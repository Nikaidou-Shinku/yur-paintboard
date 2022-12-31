use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(Session::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(Session::Uid)
              .integer()
              .not_null()
              .primary_key()
          )
          .col(ColumnDef::new(Session::PaintToken).uuid().not_null())
          .to_owned()
      ).await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(
        Table::drop()
          .table(Session::Table).to_owned()
      ).await
  }
}

#[derive(Iden)]
enum Session {
  Table,
  Uid,
  PaintToken,
}
