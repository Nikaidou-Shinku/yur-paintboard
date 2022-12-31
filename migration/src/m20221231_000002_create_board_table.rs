use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(Board::Table)
          .if_not_exists()
          .col(ColumnDef::new(Board::X).integer().not_null())
          .col(ColumnDef::new(Board::Y).integer().not_null())
          .col(ColumnDef::new(Board::Color).string().not_null())
          .col(ColumnDef::new(Board::Uid).integer().not_null())
          .col(ColumnDef::new(Board::Time).timestamp().not_null())
          .primary_key(Index::create().col(Board::X).col(Board::Y))
          .to_owned()
      ).await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(
        Table::drop()
          .table(Board::Table).to_owned()
      ).await
  }
}

#[derive(Iden)]
enum Board {
  Table,
  X,
  Y,
  Color,
  Uid,
  Time,
}
