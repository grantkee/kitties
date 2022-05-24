#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		sp_runtime::traits::{Hash, Zero},
        dispatch::{DispatchResultWithPostInfo, DispatchResult},
        traits::{Currency, ExistenceRequirement, Randomness},
        pallet_prelude::*
    };
    use frame_system::pallet_prelude::*;
    use sp_io::hashing::blake2_128;
	use scale_info::TypeInfo;


    #[cfg(feature = "std")]
    use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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

    // ACTION #3: Implementation to handle Gender type in Kitty struct.

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

        // ACTION #9: Add MaxKittyOwned constant

    }

    // Errors.
    #[pallet::error]
    pub enum Error<T> {
        // TODO Part III
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

    // ACTION #7: Remaining storage items.

    // TODO Part IV: Our pallet's genesis configuration.

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        // TODO Part III: create_kitty

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

        // TODO Part III: helper functions for dispatchable functions

        // ACTION #6: function to randomly generate DNA

        // TODO Part III: mint

        // TODO Part IV: transfer_kitty_to
    }
}
