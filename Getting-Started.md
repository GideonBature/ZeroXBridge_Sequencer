# 🚀 Getting Started

Welcome!  
This guide will help you set up your development environment quickly.

---

## 1. Start Postgres Database

If you **do not have Postgres installed locally**, you can quickly run it via Docker:

```bash
docker run -d --rm \
  --name zerox \
  -p 5434:5432 \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=zeroxdb \
  postgres
```

> ✅ This will start a Postgres container listening on `localhost:5434`, with:
>
> - Username: `postgres`
> - Password: `postgres`
> - Database: `zeroxdb`

If you already have Postgres installed, just make sure it is running and you have a database named `zeroxdb`.

---

## 2. Set up your `.env` file

Create a `.env` file at the project root if it doesn't exist, and add:

```env
DATABASE_URL=postgresql://postgres:postgres@localhost:5434/zeroxdb
```

This will allow the application and tooling (`sqlx`) to connect to your local database.

---

## 3. Expose the database for `sqlx`

Before running `sqlx` commands, ensure your terminal session has access to the database URL.

You can **load** the `.env` file like this:

```bash
source .env
```

Or you can set it manually:

```bash
export DATABASE_URL=postgresql://postgres:postgres@localhost:5434/zeroxdb
```

Now you can run `sqlx` commands like:

```bash
sqlx migrate run
sqlx migrate info
```

---

## 4. Common sqlx Commands

| Command                   | Description                  |
| :------------------------ | :--------------------------- |
| `sqlx migrate run`        | Apply all pending migrations |
| `sqlx migrate revert`     | Revert the last migration    |
| `sqlx migrate add <name>` | Create a new migration       |

---

## Notes

- The Postgres container started with Docker will automatically be removed (`--rm`) when stopped.
- If you need to stop the container manually:

  ```bash
  docker stop zerox
  ```

---
