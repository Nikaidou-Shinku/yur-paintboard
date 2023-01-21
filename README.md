## Yur Paintboard

Just paint freely!

### Build

Install `sea-orm-cli`: (Only for once)

```bash
cargo install sea-orm-cli
```

Build the project:

```bash
cargo build -r
```

perform the migrations:

```bash
DATABASE_URL="sqlite:./data.db?mode=rwc" sea-orm-cli migrate refresh
```

Setup the board:

```bash
./target/release/setup [-c <COLOR>]
```

Run the server:

```bash
./target/release/yur-paintboard
```

### Generate entity from database

```bash
sea-orm-cli generate entity \
  -u "sqlite:./data.db?mode=rwc" \
  -o src/entities
```
