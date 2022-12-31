use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(User::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(User::Uid)
              .integer()
              .not_null()
              .primary_key()
          )
          .col(ColumnDef::new(User::LuoguToken).uuid())
          .col(ColumnDef::new(User::PaintToken).uuid())
          .to_owned()
      ).await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(
        Table::drop()
          .table(User::Table).to_owned()
      ).await
  }
}

#[derive(Iden)]
enum User {
  Table,
  Uid,
  LuoguToken,
  PaintToken,
}
