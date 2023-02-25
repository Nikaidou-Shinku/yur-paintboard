## Yur Paintboard

Just paint freely!

### Build

Build the project:

```bash
cargo build -r --workspace
```

perform the migrations:

```bash
./target/release/migration -u "sqlite:./data.db?mode=rwc" refresh
```

Setup the board:

```bash
./target/release/setup [-c <COLOR>]
```

The color above can be in the format of `#f0f0f0`.

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
