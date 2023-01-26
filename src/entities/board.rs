//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.7

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "board")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub x: i32,
  #[sea_orm(primary_key, auto_increment = false)]
  pub y: i32,
  pub color: String,
  pub uid: i32,
  pub time: DateTimeLocal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
