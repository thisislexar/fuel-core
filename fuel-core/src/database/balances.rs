use crate::{
    database::{
        columns::BALANCES,
        Database,
    },
    state::{
        Error,
        IterDirection,
        MultiKey,
    },
};
use fuel_core_interfaces::common::{
    fuel_storage::MerkleRoot,
    fuel_vm::{
        crypto,
        prelude::{
            AssetId,
            ContractId,
            MerkleStorage,
            Word,
        },
    },
};
use itertools::Itertools;
use std::borrow::Cow;

impl MerkleStorage<ContractId, AssetId, Word> for Database {
    type Error = Error;

    fn insert(
        &mut self,
        parent: &ContractId,
        key: &AssetId,
        value: &Word,
    ) -> Result<Option<Word>, Error> {
        let key = MultiKey::new((parent, key));
        Database::insert(self, key.as_ref().to_vec(), BALANCES, *value)
    }

    fn remove(
        &mut self,
        parent: &ContractId,
        key: &AssetId,
    ) -> Result<Option<Word>, Error> {
        let key = MultiKey::new((parent, key));
        Database::remove(self, key.as_ref(), BALANCES)
    }

    fn get(
        &self,
        parent: &ContractId,
        key: &AssetId,
    ) -> Result<Option<Cow<Word>>, Error> {
        let key = MultiKey::new((parent, key));
        self.get(key.as_ref(), BALANCES)
    }

    fn contains_key(&self, parent: &ContractId, key: &AssetId) -> Result<bool, Error> {
        let key = MultiKey::new((parent, key));
        self.exists(key.as_ref(), BALANCES)
    }

    fn root(&mut self, parent: &ContractId) -> Result<MerkleRoot, Error> {
        let items: Vec<_> = Database::iter_all::<Vec<u8>, Word>(
            self,
            BALANCES,
            Some(parent.as_ref().to_vec()),
            None,
            Some(IterDirection::Forward),
        )
        .try_collect()?;

        let root = items
            .iter()
            .filter_map(|(key, value)| {
                (&key[..parent.len()] == parent.as_ref()).then(|| (key, value))
            })
            .sorted_by_key(|t| t.0)
            .map(|(_, value)| value.to_be_bytes());

        Ok(crypto::ephemeral_merkle_root(root).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get() {
        let balance_id: (ContractId, AssetId) =
            (ContractId::from([1u8; 32]), AssetId::new([1u8; 32]));
        let balance: Word = 100;

        let database = Database::default();
        let key: Vec<u8> = MultiKey::new(balance_id).into();
        let _: Option<Word> = database.insert(key, BALANCES, balance).unwrap();

        assert_eq!(
            MerkleStorage::<ContractId, AssetId, Word>::get(
                &database,
                &balance_id.0,
                &balance_id.1
            )
            .unwrap()
            .unwrap()
            .into_owned(),
            balance
        );
    }

    #[test]
    fn put() {
        let balance_id: (ContractId, AssetId) =
            (ContractId::from([1u8; 32]), AssetId::new([1u8; 32]));
        let balance: Word = 100;

        let mut database = Database::default();
        MerkleStorage::<ContractId, AssetId, Word>::insert(
            &mut database,
            &balance_id.0,
            &balance_id.1,
            &balance,
        )
        .unwrap();

        let returned: Word = database
            .get(MultiKey::new(balance_id).as_ref(), BALANCES)
            .unwrap()
            .unwrap();
        assert_eq!(returned, balance);
    }

    #[test]
    fn remove() {
        let balance_id: (ContractId, AssetId) =
            (ContractId::from([1u8; 32]), AssetId::new([1u8; 32]));
        let balance: Word = 100;

        let mut database = Database::default();
        database
            .insert(MultiKey::new(balance_id), BALANCES, balance)
            .unwrap();

        MerkleStorage::<ContractId, AssetId, Word>::remove(
            &mut database,
            &balance_id.0,
            &balance_id.1,
        )
        .unwrap();

        assert!(!database
            .exists(MultiKey::new(balance_id).as_ref(), BALANCES)
            .unwrap());
    }

    #[test]
    fn exists() {
        let balance_id: (ContractId, AssetId) =
            (ContractId::from([1u8; 32]), AssetId::new([1u8; 32]));
        let balance: Word = 100;

        let database = Database::default();
        database
            .insert(
                MultiKey::new(balance_id).as_ref().to_vec(),
                BALANCES,
                balance,
            )
            .unwrap();

        assert!(MerkleStorage::<ContractId, AssetId, Word>::contains_key(
            &database,
            &balance_id.0,
            &balance_id.1
        )
        .unwrap());
    }

    #[test]
    fn root() {
        let balance_id: (ContractId, AssetId) =
            (ContractId::from([1u8; 32]), AssetId::new([1u8; 32]));
        let balance: Word = 100;

        let mut database = Database::default();

        MerkleStorage::<ContractId, AssetId, Word>::insert(
            &mut database,
            &balance_id.0,
            &balance_id.1,
            &balance,
        )
        .unwrap();

        let root = MerkleStorage::<ContractId, AssetId, Word>::root(
            &mut database,
            &balance_id.0,
        );
        assert!(root.is_ok())
    }
}
