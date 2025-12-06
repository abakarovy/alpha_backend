# "Alpha Future" Hackathon, Backend
Backend service for a business assistant application, built with Rust and Actix Web.

It provides authentication, user profiles, chat with conversation history, simple business analytics, legal pages, and file download capabilities.

---

## Features

- **Health Check**
  - `GET /health`
  - Simple endpoint to verify that the server is running.

- **Authentication & User Accounts**
  - `POST /api/auth/register`
    - Registers a new user with email, password, business type, and optional profile data (including telegram_username).
    - Creates an initial session and returns a session token.
  - `POST /api/auth/login`
    - Logs in an existing user with email and password.
    - Returns a new session token on success.
  - `GET /api/auth/check-user?email={email}`
    - Checks if a user with the given email already exists.
  - `GET /api/auth/check-telegram-username?telegram_username={username}`
    - Checks if a user with the given Telegram username already exists.
  - `GET /api/auth/check-token?token={token}`
    - Validates whether a session token is present and not expired.

- **User Profile**
  - `GET /api/auth/profile?token={token}`
    - Returns the authenticated user profile (without password), including:
      - `id`, `email`, `business_type`, `created_at`
      - Optional: `full_name`, `nickname`, `phone`, `country`, `gender`, `telegram_username`, `profile_picture`
  - `PUT /api/auth/profile?token={token}`
    - Updates the authenticated user's profile fields:
      - `business_type`, `full_name`, `nickname`, `phone`, `country`, `gender`, `telegram_username`, `profile_picture`
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
  - `GET /api/analytics/weekly-trends`
  - `POST /api/analytics/weekly-trends`
    - Get or upsert weekly trends for the current week:
      - Top trend (1st place) with title, increase percentage, and request percentage
      - 2nd place with title and increase percentage
      - Top 3 geographic trends (country and increase percentage)
  - `GET /api/analytics/ai-analytics`
  - `POST /api/analytics/ai-analytics`
    - Get or create AI analytics data:
      - Increase percentage
      - Trend description
      - Array of competitiveness level data points (minimum 5 values for graph)
  - `GET /api/analytics/niches-month`
  - `POST /api/analytics/niches-month`
    - Get or upsert niches for the current month:
      - Array of niches with title and change percentage (positive = growth, negative = decline)
  - `GET /api/analytics/top-trend`
  - `POST /api/analytics/top-trend`
    - Get or upsert a "top trend" analytics record (legacy, for backward compatibility).
  - `GET /api/analytics/popularity`
  - `POST /api/analytics/popularity`
    - Get or upsert popularity trend records (legacy, for backward compatibility).

- **Telegram Users**
  - `POST /api/telegram/users`
    - Creates a new Telegram user or returns existing one if already registered.
    - Automatically registers Telegram users when they start the bot.
    - Body: `telegram_user_id` (required), `telegram_username`, `first_name`, `last_name` (all optional)
  - `GET /api/telegram/users/{telegram_user_id}`
    - Retrieves a Telegram user by their Telegram user ID.
  - `POST /api/telegram/users/{telegram_user_id}/link`
    - Links a Telegram user to a main user account.
    - Body: `user_id` (required)

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
  "full_name": "Jane Doe",
  "telegram_username": "janedoe"
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

### Check if Email Exists

```http
GET /api/auth/check-user?email=user@example.com
```

Response:
```json
{
  "exists": true,
  "profile_picture": "file-uuid-here"
}
```

Note: `profile_picture` will be `null` if the user doesn't have a profile picture set.

### Check if Telegram Username Exists

```http
GET /api/auth/check-telegram-username?telegram_username=janedoe
```

Response:
```json
{
  "exists": true
}
```

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
  "country": "US",
  "telegram_username": "newusername"
}
```

### Get Weekly Trends

```http
GET /api/analytics/weekly-trends
```

### Upsert Weekly Trends

```http
POST /api/analytics/weekly-trends
Content-Type: application/json

{
  "current_top": {
    "title": "Gaming laptops",
    "increase": 92.0,
    "request_percent": 18.0
  },
  "second_place": {
    "title": "Online education",
    "increase": 76.0
  },
  "geo_trends": [
    { "country": "Belgium", "increase": 54.0 },
    { "country": "Netherlands", "increase": 48.0 },
    { "country": "Germany", "increase": 42.0 }
  ]
}
```

### Get AI Analytics

```http
GET /api/analytics/ai-analytics
```

### Create AI Analytics

```http
POST /api/analytics/ai-analytics
Content-Type: application/json

{
  "increase": 10.0,
  "description": "Online education trend can be used to increase the brand as a source of benefit",
  "level_of_competitiveness": [25.5, 30.2, 35.8, 28.4, 32.1, 40.0, 38.7]
}
```

### Get Niches of the Month

```http
GET /api/analytics/niches-month
```

### Upsert Niches of the Month

```http
POST /api/analytics/niches-month
Content-Type: application/json

{
  "niches": [
    { "title": "Beauty", "change": 34.0 },
    { "title": "Food Delivery", "change": -6.0 },
    { "title": "Fitness", "change": 28.5 }
  ]
}
```

### Create or Get Telegram User

```http
POST /api/telegram/users
Content-Type: application/json

{
  "telegram_user_id": 123456789,
  "telegram_username": "janedoe",
  "first_name": "Jane",
  "last_name": "Doe"
}
```

Response (201 Created or 200 OK if exists):
```json
{
  "id": "uuid-here",
  "telegram_user_id": 123456789,
  "telegram_username": "janedoe",
  "first_name": "Jane",
  "last_name": "Doe",
  "created_at": "2024-01-01T00:00:00Z",
  "user_id": null
}
```

### Get Telegram User by ID

```http
GET /api/telegram/users/123456789
```

### Link Telegram User to Main Account

```http
POST /api/telegram/users/123456789/link
Content-Type: application/json

{
  "user_id": "main-user-uuid"
}
```

---

## Development Notes

- The server uses `NormalizePath` middleware to normalize trailing slashes.
- CORS is configured as permissive by default to simplify frontend integration.
- Conversation history is stored in memory keyed by user ID, while user and session data are persisted in SQLite.
