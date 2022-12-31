use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(Auth::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(Auth::Uid)
              .integer()
              .not_null()
              .primary_key()
          )
          .col(ColumnDef::new(Auth::Session).uuid().not_null())
          .col(ColumnDef::new(Auth::LuoguToken).uuid().not_null())
          .to_owned()
      ).await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(
        Table::drop()
          .table(Auth::Table).to_owned()
      ).await
  }
}

#[derive(Iden)]
enum Auth {
  Table,
  Uid,
  Session,
  LuoguToken,
}
