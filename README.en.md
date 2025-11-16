# alpha_backend
Backend service for a business assistant application, built with Rust and Actix Web.

It provides authentication, user profiles, chat with conversation history, simple business analytics, legal pages, and file download capabilities.

---

## Features

- **Health Check**
  - `GET /health`
  - Simple endpoint to verify that the server is running.

- **Authentication & User Accounts**
  - `POST /api/auth/register`
    - Registers a new user with email, password, business type, and optional profile data.
    - Creates an initial session and returns a session token.
  - `POST /api/auth/login`
    - Logs in an existing user with email and password.
    - Returns a new session token on success.
  - `GET /api/auth/check-user?email={email}`
    - Checks if a user with the given email already exists.
  - `GET /api/auth/check-token?token={token}`
    - Validates whether a session token is present and not expired.

- **User Profile**
  - `GET /api/auth/profile?token={token}`
    - Returns the authenticated user profile (without password), including:
      - `id`, `email`, `business_type`, `created_at`
      - Optional: `full_name`, `nickname`, `phone`, `country`, `gender`
  - `PUT /api/auth/profile?token={token}`
    - Updates the authenticated user’s profile fields:
      - `business_type`, `full_name`, `nickname`, `phone`, `country`, `gender`
    - Returns the updated profile.

- **Chat & Conversations**
  - `POST /api/chat/message`
    - Sends a chat message to the assistant and returns a response.
    - Uses stored conversation history keyed by user ID.
  - `GET /api/chat/conversations/{user_id}`
    - Lists conversations for a given user.
  - `GET /api/chat/history/{conversation_id}`
    - Returns the message history for a specific conversation.

- **Analytics**
  - `GET /api/analytics/top-trend`
  - `POST /api/analytics/top-trend`
    - Get or upsert a “top trend” analytics record.
  - `GET /api/analytics/popularity`
  - `POST /api/analytics/popularity`
    - Get or upsert popularity trend records.

- **Files**
  - `GET /api/files/{id}`
    - Download a stored file by its ID.

- **Legal**
  - `GET /privacy-policy`
    - Returns the privacy policy page content.

---

## Tech Stack

- **Language:** Rust
- **Web Framework:** Actix Web
- **Database:** SQLite (via `sqlx`)
- **Other:** `bcrypt` for password hashing, `uuid` for identifiers, `chrono` for timestamps, `dotenvy` for environment variables.

---

## Getting Started

### Prerequisites

- Rust toolchain (stable) installed
- SQLite installed (or any environment where a SQLite file DB is acceptable)

### 1. Clone the repository

```bash
git clone https://github.com/your-org/alpha_backend.git
cd alpha_backend
```

### 2. Configure environment variables

Create a `.env` file in the project root if it does not already exist:

```env
# Port the server will listen on (optional, default: 3000)
PORT=3000

# SQLite database URL
DATABASE_URL=sqlite://app.db
```

If `DATABASE_URL` is not set, the app defaults to `sqlite://app.db` in the project root.

### 3. Database initialization

On first run, the app will create and migrate the SQLite database (tables for `users`, `sessions`, `analytics_trends`, etc.) automatically using `sqlx` queries in `db.rs`.

You generally don’t need to run migrations manually; just ensure the process can write to `app.db`.

### 4. Run the server

```bash
cargo run
```

By default the server listens on:

```text
http://0.0.0.0:3000
```

If `PORT` is set, it will listen on that port instead.

---

## Example Requests

### Register a User

```http
POST /api/auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "strong-password",
  "business_type": "ecommerce",
  "full_name": "Jane Doe"
}
```

### Login

```http
POST /api/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "strong-password"
}
```

Response includes a `token` you can pass as `?token=` when calling profile endpoints.

### Get Profile

```http
GET /api/auth/profile?token=SESSION_TOKEN
```

### Update Profile

```http
PUT /api/auth/profile?token=SESSION_TOKEN
Content-Type: application/json

{
  "full_name": "New Name",
  "phone": "+123456789",
  "country": "US"
}
```

---

## Development Notes

- The server uses `NormalizePath` middleware to normalize trailing slashes.
- CORS is configured as permissive by default to simplify frontend integration.
- Conversation history is stored in memory keyed by user ID, while user and session data are persisted in SQLite.
