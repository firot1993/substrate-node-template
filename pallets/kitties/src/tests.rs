use frame_support::{assert_err, assert_ok};

use crate::{Error, mock::*};


#[test]
fn test_create_kitty() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		System::set_block_number(1);
		assert_ok!(KittiesModule::create_kitty(Origin::signed(1)));
		assert_ok!(KittiesModule::create_kitty(Origin::signed(1)));
		assert_ok!(KittiesModule::create_kitty(Origin::signed(1)));
		assert_err!(KittiesModule::create_kitty(Origin::signed(2)), Error::<Test>::NotEnoughBalanceForStake);
		let to_owned = KittiesModule::kittles_owned(1);
		assert_eq!(to_owned.get(0), Some(&1u32));
		assert_err!(KittiesModule::create_kitty(Origin::signed(1)), Error::<Test>::ExceedMaxKittyOwned);
	});
}


#[test]
fn test_set_price() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		System::set_block_number(1);
		assert_ok!(KittiesModule::create_kitty(Origin::signed(1)));
		assert_ok!(KittiesModule::set_price(Origin::signed(1),1, Option::Some(1)));
		assert_err!(KittiesModule::set_price(Origin::signed(1),2, Option::Some(1)), Error::<Test>::KittyNotExist);
		assert_err!(KittiesModule::set_price(Origin::signed(2),1, Option::Some(1)), Error::<Test>::NotKittyOwner);
	});
}

#[test]
fn test_transfer() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		System::set_block_number(1);
		assert_ok!(KittiesModule::create_kitty(Origin::signed(1)));
		assert_err!(KittiesModule::transfer(Origin::signed(1), 2, 1), Error::<Test>::NotEnoughBalanceForStake);
		assert_err!(KittiesModule::transfer(Origin::signed(2), 2, 1), Error::<Test>::TransferToSelf);
		assert_err!(KittiesModule::transfer(Origin::signed(1), 2, 1), Error::<Test>::NotKittyOwner);
		assert_ok!(KittiesModule::transfer(Origin::signed(2), 1, 1));
		assert_ok!(KittiesModule::create_kitty(Origin::signed(3)));
		assert_ok!(KittiesModule::create_kitty(Origin::signed(3)));
		assert_ok!(KittiesModule::create_kitty(Origin::signed(3)));
		assert_err!(KittiesModule::transfer(Origin::signed(1), 3, 1), Error::<Test>::ExceedMaxKittyOwned);
	});
}

#[test]
fn test_buy_kitty() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		System::set_block_number(1);
		assert_ok!(KittiesModule::create_kitty(Origin::signed(1)));
		assert_ok!(KittiesModule::create_kitty(Origin::signed(3)));
		assert_ok!(KittiesModule::create_kitty(Origin::signed(3)));
		assert_ok!(KittiesModule::create_kitty(Origin::signed(3)));
		assert_err!(KittiesModule::buy_kitty(Origin::signed(2),1, 6), Error::<Test>::KittyNotForSale);
		assert_ok!(KittiesModule::set_price(Origin::signed(1),1, Option::Some(10)));
		assert_err!(KittiesModule::buy_kitty(Origin::signed(1),1, 6), Error::<Test>::BuyerIsKittyOwner);
		assert_err!(KittiesModule::buy_kitty(Origin::signed(2),1, 6), Error::<Test>::KittyBidPriceTooLow);
		assert_err!(KittiesModule::buy_kitty(Origin::signed(2),1, 12), Error::<Test>::NotEnoughBalance);
		assert_err!(KittiesModule::buy_kitty(Origin::signed(3),1, 10), Error::<Test>::ExceedMaxKittyOwned);
	});
}


#[test]
fn test_breed_kitty() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		System::set_block_number(1);
		assert_ok!(KittiesModule::create_kitty(Origin::signed(1)));
		assert_ok!(KittiesModule::create_kitty(Origin::signed(1)));
		assert_ok!(KittiesModule::breed_kitty(Origin::signed(1),1,2));
		assert_err!(KittiesModule::breed_kitty(Origin::signed(1),3,4),Error::<Test>::KittyNotExist);
		assert_err!(KittiesModule::breed_kitty(Origin::signed(1),1,2), Error::<Test>::ExceedMaxKittyOwned);
		assert_ok!(KittiesModule::transfer(Origin::signed(1), 3, 2));
		assert_err!(KittiesModule::breed_kitty(Origin::signed(1),1,2), Error::<Test>::NotKittyOwner);
	});
}

