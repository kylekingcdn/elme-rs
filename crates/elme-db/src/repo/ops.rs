mod insert;

pub use insert::{
    InsertMany,
    InsertOne,
};

pub mod prepare {
    pub use super::{
        insert::{
            PrepInsertMany,
            PrepInsertOne,
        },
    };
}
