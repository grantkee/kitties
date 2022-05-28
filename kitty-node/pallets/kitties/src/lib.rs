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
	};
	use frame_system::pallet_prelude::{OriginFor, *};
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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// TODO Part III
	}

	// Storage item to keep a count of all existing Kitties.
	#[pallet::storage]
	#[pallet::getter(fn count_for_kitties)]
	/// Keeps track of the total number of Kitties in existence.
	pub(super) type CountForKitties<T: Config> = StorageValue<_, u64, ValueQuery>;

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

			// TODO: Deposit `Created` event

			Ok(())
		}

		// TODO Part IV: set_price

		// TODO Part IV: transfer

		// TODO Part IV: buy_kitty

		// TODO Part IV: breed_kitty
	}

	//** Our helper functions.**//

	impl<T: Config> Pallet<T> {
		pub fn gen_gender() -> Gender {
			let random = T::KittyRandomness::random(&b"gender"[..]).0;
			match random.as_ref()[0] % 2 {
				0 => Gender::Male,
				_ => Gender::Female,
			}
		}

		pub fn gen_dna() -> [u8; 16] {
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

		// TODO Part IV: transfer_kitty_to
	}
}
