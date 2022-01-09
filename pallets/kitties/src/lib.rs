#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::{
		traits::{ Randomness, Currency, tokens::ExistenceRequirement },
		transactional
	};

	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};
	use frame_support::traits::{LockableCurrency, LockIdentifier, WithdrawReasons};
	use frame_support::traits::tokens::AssetId;

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	const KITTIESID: LockIdentifier = *b"kittiess";

	// Struct for holding Kitty information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		pub dna: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: AccountOf<T>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
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

		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		#[pallet::constant]
		type MaxKittyOwned: Get<u32>;

		#[pallet::constant]
		type NeedLockBalance: Get<u32>;

		type KittyIndex: AssetId;

		fn get_kitty_index_from_u64(kitty_id: u64) ->Self::KittyIndex;
		fn get_u64_from_kitty_index(kitty_index: Self::KittyIndex) ->u64;
	}

	// Errors.
	#[pallet::error]
	pub enum Error<T> {
		/// Handles arithmetic overflow when incrementing the Kitty counter.
		KittyCntOverflow,
		/// An account cannot own more Kitties than `MaxKittyCount`.
		ExceedMaxKittyOwned,
		/// Buyer cannot be the owner.
		BuyerIsKittyOwner,
		/// Cannot transfer a kitty to its owner.
		TransferToSelf,
		/// Handles checking whether the Kitty exists.
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

	// Events.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new Kitty was sucessfully created. \[sender, kitty_id\]
		Created(T::AccountId, T::KittyIndex),
		/// Kitty price was sucessfully set. \[sender, kitty_id, new_price\]
		PriceSet(T::AccountId, T::KittyIndex, Option<BalanceOf<T>>),
		/// A Kitty was sucessfully transferred. \[from, to, kitty_id\]
		Transferred(T::AccountId, T::AccountId, T::KittyIndex),
		/// A Kitty was sucessfully bought. \[buyer, seller, kitty_id, bid_price\]
		Bought(T::AccountId, T::AccountId, T::KittyIndex, BalanceOf<T>),
	}


	#[pallet::storage]
	#[pallet::getter(fn kitty_cnt)]
	pub(super) type KittyCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub(super) type Kitties<T:Config> = StorageMap<
	_, Twox64Concat, T::KittyIndex, Kitty<T>, >;

	#[pallet::storage]
	#[pallet::getter(fn kittles_owned)]
	pub(super) type KittiesOwned<T:Config> = StorageMap<
		_, Twox64Concat, T::AccountId,
		BoundedVec<T::KittyIndex, T::MaxKittyOwned>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub kitties: Vec<(T::AccountId, [u8; 16], Gender)>,
	}

	// Required to implement default for GenesisConfig.
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { kitties: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// When building a kitty from genesis config, we require the dna and gender to be supplied.
			for (acct, dna, gender) in &self.kitties {
				let _ = <Pallet<T>>::mint(acct, Some(dna.clone()), Some(gender.clone()));
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(100)]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			log::info!("Currency is {:?}.",T::Currency::free_balance(&sender));
			ensure!(T::Currency::free_balance(&sender) >= T::NeedLockBalance::get().into(), Error::<T>::NotEnoughBalance);
			let kitty_id = Self::mint(&sender, None, None)?;
			T::Currency::set_lock(
				KITTIESID,
				&sender,
				T::NeedLockBalance::get().into(),
				WithdrawReasons::all(),
			);
			log::info!("A kitty is born with ID: {:?}.", kitty_id);
			Self::deposit_event(Event::Created(sender, kitty_id));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn set_price(
			origin: OriginFor<T>,
			kitty_id: T::KittyIndex,
			new_price: Option<BalanceOf<T>>
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_kitty_owner(&kitty_id, &sender)?, <Error<T>>::NotKittyOwner);
			let mut kitty: Kitty<T> = Self::kitties(&kitty_id).ok_or(Error::<T>::KittyNotExist)?;
			kitty.price = new_price.clone();
			Kitties::<T>::insert(kitty_id, kitty);
			Self::deposit_event(Event::PriceSet(sender, kitty_id, new_price));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn transfer(origin: OriginFor<T>, to: T::AccountId, kitty_id: T::KittyIndex) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_kitty_owner(&kitty_id, &sender)?, <Error<T>>::NotKittyOwner);
			let to_owned = Self::kittles_owned(&to);
			ensure!((to_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);
			Self::transfer_kitty_to(&kitty_id, &to)?;
			Self::deposit_event(Event::Transferred(sender, to, kitty_id));
			Ok(())
		}

		// buy_kitty
		#[transactional]
		#[pallet::weight(100)]
		pub fn buy_kitty(
			origin: OriginFor<T>,
			kitty_id: T::KittyIndex,
			bid_price: BalanceOf<T>
		) -> DispatchResult {

			let buyer = ensure_signed(origin)?;
			let kitty = Self::kitties(&kitty_id).ok_or(Error::<T>::KittyNotExist)?;
			ensure!(kitty.owner != buyer, <Error<T>>::BuyerIsKittyOwner);

			if let Some(ask_price) = kitty.price {
				ensure!(ask_price <= bid_price, <Error<T>>::KittyBidPriceTooLow);
			} else {
				Err(Error::<T>::KittyNotForSale)?;
			}
			ensure!(T::Currency::free_balance(&buyer) >= bid_price, <Error<T>>::NotEnoughBalance);

			let to_owned = <KittiesOwned<T>>::get(&buyer);
			ensure!((to_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			let seller = kitty.owner.clone();
			T::Currency::transfer(&buyer, &seller, bid_price, ExistenceRequirement::KeepAlive)?;

			Self::transfer_kitty_to(&kitty_id, &buyer)?;

			Self::deposit_event(Event::Bought(buyer, seller, kitty_id, bid_price));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn breed_kitty(
			origin: OriginFor<T>,
			parent1: T::KittyIndex,
			parent2: T::KittyIndex
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Check: Verify `sender` owns both kitties (and both kitties exist).
			ensure!(Self::is_kitty_owner(&parent1, &sender)?, <Error<T>>::NotKittyOwner);
			ensure!(Self::is_kitty_owner(&parent2, &sender)?, <Error<T>>::NotKittyOwner);

			let new_dna = Self::breed_dna(&parent1, &parent2)?;

			Self::mint(&sender, Some(new_dna), None)?;

			Ok(())
		}
	}

	//** Our helper functions.**//

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
				T::KittyRandomness::random_seed(),
					<frame_system::Pallet<T>>::block_number(),
				);
			payload.using_encoded(blake2_128)
		}

		fn mint(owner: &T::AccountId,
			dna: Option<[u8; 16]>, gender: Option<Gender>) -> Result<T::KittyIndex, Error<T>>{
			let kitty = Kitty::<T>{
				dna: dna.unwrap_or_else(Self::gen_dna),
				price: None,
				gender: gender.unwrap_or_else(Self::gen_gender),
				owner: owner.clone(),
			};
			let new_cnt = Self::kitty_cnt().checked_add(1).ok_or(Error::<T>::KittyCntOverflow)?;
			let kitty_id = T::get_kitty_index_from_u64(new_cnt.clone());
			KittiesOwned::<T>::try_mutate(&owner, |kitty_vec|{
				kitty_vec.try_push(kitty_id)
			}).map_err(|_| Error::<T>::ExceedMaxKittyOwned)?;
			Kitties::<T>::insert(kitty_id, kitty);
			KittyCnt::<T>::put(new_cnt);
			Ok(kitty_id)
		}

		fn transfer_kitty_to( kitty_id: &T::KittyIndex, to: &T::AccountId) -> Result<(), Error<T>>{
			let mut kitty: Kitty<T> = Self::kitties(&kitty_id).ok_or(Error::<T>::KittyNotExist)?;
			let prev_owner = kitty.owner.clone();
			if prev_owner == *to {
				return Err(Error::<T>::TransferToSelf);
			}
			<KittiesOwned<T>>::try_mutate(&prev_owner, |owned| {
				if let Some(ind) = owned.iter().position(|&id| id == *kitty_id) {
					owned.swap_remove(ind);
					return Ok(());
				}
				Err(())
			}).map_err(|_| Error::<T>::KittyNotExist)?;

			// Update the kitty owner
			kitty.owner = to.clone();
			// Reset the ask price so the kitty is not for sale until `set_price()` is called
			// by the current owner.
			kitty.price = None;

			<Kitties<T>>::insert(kitty_id, kitty);

			<KittiesOwned<T>>::try_mutate(to, |vec| {
				vec.try_push(*kitty_id)
			}).map_err(|_| Error::<T>::ExceedMaxKittyOwned)?;

			Ok(())
		}

		pub fn is_kitty_owner(kitty_id: &T::KittyIndex, acct: &T::AccountId) -> Result<bool, Error<T>> {
			match Self::kitties(kitty_id) {
				Some(kitty) => Ok(kitty.owner == *acct),
				None => Err(Error::<T>::KittyNotExist)
			}
		}

		pub fn breed_dna(parent1: &T::KittyIndex, parent2: &T::KittyIndex) -> Result<[u8; 16], Error<T>> {
			let dna1 = Self::kitties(parent1).ok_or(Error::<T>::KittyNotExist)?.dna;
			let dna2 = Self::kitties(parent2).ok_or(Error::<T>::KittyNotExist)?.dna;

			let mut new_dna = Self::gen_dna();
			for i in 0..new_dna.len() {
				new_dna[i] = (new_dna[i] & dna1[i]) | (!new_dna[i] & dna2[i]);
			}
			Ok(new_dna)
		}
	}
}
