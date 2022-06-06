#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::{DispatchResult, DispatchResultWithPostInfo},
		pallet_prelude::{ValueQuery, *},
		sp_runtime::traits::{Hash, Zero},
		traits::{Currency, ExistenceRequirement, Randomness},
		Twox64Concat,
		transactional,
	};
	use frame_system::{pallet_prelude::{OriginFor, *}, ensure_signed};
	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	// Struct for individual Kitty info
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Kitty<T: Config> {
		pub dna: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: AccountOf<T>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Gender {
		Male,
		Female,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types it depends on.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the Kitties pallet.
		type Currency: Currency<Self::AccountId>;

		/// KittyRandomness type bound by Randomness trait.
		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		/// Constant for maximum Kitties owned.
		#[pallet::constant]
		type MaxKittyOwned: Get<u32>;
	}

	// Errors.
	#[pallet::error]
	pub enum Error<T> {
		/// Handle arithmetic overflow when incrementing the Kitty counter.
		CountForKittiesOverflow,
		/// Account limit exceeded for `MaxKittyCount`.
		ExceedMaxKittyOwned,
		/// New kitty is not unique and already exists.
		KittyExists,
		/// Buyer cannot be the owner.
		BuyerIsKittyOwner,
		/// Cannot transfer a kitty to its owner.
		TransferToSelf,
		/// This kitty doesn't exist
		KittyNotExist,
		/// Handles checking that the Kitty is owned by the account transferring, buying or setting a price for it.
		NotKittyOwner,
		/// Ensures the Kitty is for sale.
		KittyNotForSale,
		/// Ensures that the buying price is greater than the asking price.
		KittyBidPriceTooLow,
		/// Ensures that an account has enough funds to purchase a Kitty.
		NotEnoughBalance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Kitty was successfully created. [snder, kitty_id]
		Created(T::AccountId, T::Hash),
		/// Kitty price was successfully set. [sender, kitty_id, new_price]
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		/// Kitty was successfully transferred to a different owner. [from, to, kitty_id]
		Transferred(T::AccountId, T::AccountId, T::Hash),
		/// Kitty was successfully bought.
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
	}

	#[pallet::storage]
	#[pallet::getter(fn count_for_kitties)]
	/// Keeps track of the total number of Kitties in existence.
	pub(super) type CountForKitties<T: Config> = StorageValue<_, u64, ValueQuery>;
	
	/// Storage item to keep a count of all existing Kitties.
	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub(super) type Kitties<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Kitty<T>>;

	#[pallet::storage]
	#[pallet::getter(fn kitties_owned)]
	pub(super) type KittiesOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::Hash, T::MaxKittyOwned>,
		ValueQuery,
	>;

	// TODO Part IV: Our pallet's genesis configuration.

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100)]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let kitty_id = Self::mint(&sender, None, None)?;

			// log to console
			log::info!("A kitty is born with ID: {:?}", kitty_id);

			Self::deposit_event(Event::Created(sender, kitty_id));

			Ok(())
		}

		#[pallet::weight(100)]
		pub fn set_price(
			origin: OriginFor<T>,
			kitty_id: T::Hash,
			new_price: Option<BalanceOf<T>>
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(Self::is_kitty_owner(&kitty_id, &sender)?, <Error<T>>::NotKittyOwner);

			let mut kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			kitty.price = new_price.clone();
			<Kitties<T>>::insert(&kitty_id, kitty);

			// deposit "PriceSet" event
			Self::deposit_event(Event::PriceSet(sender, kitty_id, new_price));

			Ok(())
		}

		#[pallet::weight(200)]
		pub fn transer(
			origin: OriginFor<T>,
			new_owner: T::AccountId,
			kitty_id: T::Hash,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			// ensure kitty exists and is called by the current owner
			ensure!(Self::is_kitty_owner(&kitty_id, &from)?, <Error<T>>::NotKittyOwner);

			// verify the kitty is not transferring to current owner
			ensure!(from != new_owner, <Error<T>>::TransferToSelf);

			// verify new owner has capacity for another kitty
			let to_owned = <KittiesOwned<T>>::get(&new_owner);
			ensure!((to_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			Self::transfer_kitty_to(&kitty_id, &new_owner)?;

			Ok(())
		}

		#[transactional]
		#[pallet::weight(100)]
		pub fn buy_kitty(
			origin: OriginFor<T>,
			kitty_id: T::Hash,
			bid_price: BalanceOf<T>
		) -> DispatchResult {
			let buyer = ensure_signed(origin)?;

			// check the kitty exists and buyer is not the current kitty owner
			let kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;
			ensure!(kitty.owner != buyer, <Error<T>>::BuyerIsKittyOwner);

			// ensure kitty is for sale
			// ensure ask_price <= bid_price
			if let Some(ask_price) = kitty.price {
				ensure!(ask_price <= bid_price, <Error<T>>::KittyBidPriceTooLow);
			} else {
				Err(<Error<T>>::KittyNotForSale)?;
			}

			// ensure buyer has enough free balance to purchase kitty
			ensure!(T::Currency::free_balance(&buyer) >= bid_price, <Error<T>>::NotEnoughBalance);

			// ensure buyer has capacity for another kitty
			let to_owned = <KittiesOwned<T>>::get(&buyer);
			ensure!((to_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			let seller = kitty.owner.clone();

			// transfer amount from buyer to seller
			T::Currency::transfer(&buyer, &seller, bid_price, ExistenceRequirement::KeepAlive)?;

			// transfer kitty from buyer to seller
			Self::transfer_kitty_to(&kitty_id, &buyer)?;

			// deposit event
			Self::deposit_event(Event::Bought(buyer, seller, kitty_id, bid_price));

			Ok(())
		}

		/// Breed kitty.
		/// 
		/// Breed two kitties to create a new generation.
		pub fn breed_kitty(
			origin: OriginFor<T>,
			parent1: T::Hash,
			parent2: T::Hash
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// verify `sender` owns both kitties
			// and both parents exist
			ensure!(Self::is_kitty_owner(&parent1, &sender)?, <Error<T>>::NotKittyOwner);
			ensure!(Self::is_kitty_owner(&parent2, &sender)?, <Error<T>>::NotKittyOwner);

			let new_dna = Self::breed_dna(&parent1, &parent2)?;

			Self::mint(&sender, Some(new_dna), None)?;

		    Ok(())

		}
	}

	//** helper functions **//

	impl<T: Config> Pallet<T> {
		fn gen_gender() -> Gender {
			let random = T::KittyRandomness::random(&b"gender"[..]).0;
			match random.as_ref()[0] % 2 {
				0 => Gender::Male,
				_ => Gender::Female,
			}
		}

		fn gen_dna() -> [u8; 16] {
			let payload = (
				T::KittyRandomness::random(&b"dna"[..]).0,
				<frame_system::Pallet<T>>::extrinsic_index().unwrap_or_default(),
				<frame_system::Pallet<T>>::block_number(),
			);
			payload.using_encoded(blake2_128)
		}

		pub fn mint(
			owner: &T::AccountId,
			dna: Option<[u8; 16]>,
			gender: Option<Gender>,
		) -> Result<T::Hash, Error<T>> {
			// create new kitty struct
			let kitty = Kitty::<T> {
				dna: dna.unwrap_or_else(Self::gen_dna),
				price: None,
				gender: gender.unwrap_or_else(Self::gen_gender),
				owner: owner.clone(),
			};

			// create new kitty id
			let kitty_id = T::Hashing::hash_of(&kitty);

			// ensure count of all kitties in existence doesn't overflow
			let new_count = Self::count_for_kitties()
				.checked_add(1)
				.ok_or(<Error<T>>::CountForKittiesOverflow)?;

			// ensure kitty doesn't already exist in Kitties StorageMap
			ensure!(Self::kitties(&kitty_id) == None, <Error<T>>::KittyExists);

			// ensure owner doesn't have too many kitties
			<KittiesOwned<T>>::try_mutate(&owner, |kitty_vec| kitty_vec.try_push(kitty_id))
				.map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			// update storage items
			<Kitties<T>>::insert(kitty_id, kitty);
			<CountForKitties<T>>::put(new_count);
			Ok(kitty_id)
		}

		pub fn is_kitty_owner(
			kitty_id: &T::Hash,
			account: &T::AccountId,
		) -> Result<bool, Error<T>> {
			match Self::kitties(kitty_id) {
				Some(kitty) => Ok(kitty.owner == *account),
				None => Err(<Error<T>>::KittyNotExist)
			}
		}

		#[transactional]
		pub fn transfer_kitty_to(
			kitty_id: &T::Hash,
			new_owner: &T::AccountId,
		) -> Result<(), Error<T>> {
			let mut kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			let prev_owner = kitty.owner.clone();

			// remove kitty_id from KittiesOwned vec of prev_owner
			<KittiesOwned<T>>::try_mutate(&prev_owner, |owned| {
				if let Some(ind) = owned.iter().position(|&id| id == *kitty_id) {
					owned.swap_remove(ind);
					return Ok(());
				}
				Err(())
			}).map_err(|_| <Error<T>>::KittyNotExist)?;

			// update kitt owner
			kitty.owner = new_owner.clone();

			// reset the price so the kitty is not for sale until `set_price()` is called by new_owner
			kitty.price = None;

			<Kitties<T>>::insert(kitty_id, kitty);
			<KittiesOwned<T>>::try_mutate(new_owner, |vec| {
				vec.try_push(*kitty_id)
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			Ok(())
		}

		pub fn breed_dna(
			parent1: &T::Hash,
			parent2: &T::Hash,
		) -> Result<[u8; 16], Error<T>> {
			let dna1 = Self::kitties(parent1).ok_or(<Error<T>>::KittyNotExist)?.dna;
			let dna2 = Self::kitties(parent2).ok_or(<Error<T>>::KittyNotExist)?.dna;

			let mut new_dna = Self::gen_dna();

			for i in 0..new_dna.len() {
				new_dna[i] = (new_dna[i] & dna1[i] | (!new_dna[i] & dna2[i]));
			}

			Ok(new_dna)
		}
	}
}
