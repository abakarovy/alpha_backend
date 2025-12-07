# Guide: Fixing Conversation Synchronization

## Problem

Conversations are not synced between regular users and Telegram users because:

1. **Username mismatch**: 
   - In `users` table: `telegram_username = @abakarovy` (with @)
   - In `telegram_users` table: `telegram_username = abakarovyl` (different name!)
   
2. **No direct link**: 
   - `telegram_users.user_id` is NULL (no connection to main user account)

## Solution

### Option 1: Manually Link Users (Recommended for existing data)

Use the API endpoint to link the Telegram user to the main user account:

```bash
POST /api/telegram/users/{telegram_user_id}/link
Content-Type: application/json

{
  "user_id": "0b907417-9a2a-455b-a41a-85059cd2f1b1"
}
```

For your case:
```bash
POST /api/telegram/users/1002294944/link
Content-Type: application/json

{
  "user_id": "0b907417-9a2a-455b-a41a-85059cd2f1b1"
}
```

This will update `telegram_users.user_id` to link them together.

### Option 2: Fix via SQL (Direct Database Access)

Connect to your database and run:

```sql
-- Link telegram user to main user
UPDATE telegram_users 
SET user_id = '0b907417-9a2a-455b-a41a-85059cd2f1b1'
WHERE telegram_user_id = 1002294944;
```

### Option 3: Fix Username Mismatch

If usernames should match, update them:

```sql
-- Option A: Update users table to match telegram_users
UPDATE users 
SET telegram_username = 'abakarovyl'
WHERE id = '0b907417-9a2a-455b-a41a-85059cd2f1b1';

-- Option B: Update telegram_users to match users
UPDATE telegram_users 
SET telegram_username = 'abakarovy'
WHERE telegram_user_id = 1002294944;
```

Then link them:
```sql
UPDATE telegram_users 
SET user_id = '0b907417-9a2a-455b-a41a-85059cd2f1b1'
WHERE telegram_user_id = 1002294944;
```

## After Fixing

Once users are linked:

1. **All existing conversations** created by the main user will be visible to Telegram user
2. **All new conversations** created by either platform will be synced
3. The system uses normalized username matching (case-insensitive, @-prefix removed)

## Verification

Check if users are properly linked:

```sql
SELECT 
    u.id as main_user_id,
    u.email,
    u.telegram_username as user_telegram_username,
    tu.telegram_user_id,
    tu.telegram_username as telegram_user_username,
    tu.user_id as linked_user_id
FROM users u
LEFT JOIN telegram_users tu ON tu.user_id = u.id OR 
    LOWER(TRIM(REPLACE(u.telegram_username, '@', ''))) = LOWER(TRIM(REPLACE(tu.telegram_username, '@', '')))
WHERE u.email = 'maga@maga.com' OR tu.telegram_user_id = 1002294944;
```

Check conversations:

```sql
SELECT 
    c.id,
    c.title,
    c.user_id,
    u.email,
    u.telegram_username,
    c.created_at
FROM conversations c
LEFT JOIN users u ON c.user_id = u.id
WHERE c.user_id = '0b907417-9a2a-455b-a41a-85059cd2f1b1'
ORDER BY c.created_at DESC;
```

## Automatic Linking

The system now supports automatic linking when:

1. **Username matches** (normalized, case-insensitive, @-prefix removed)
2. Telegram user is created/updated via `/api/telegram/users`
3. Username in `users.telegram_username` matches `telegram_users.telegram_username`

However, in your case, the usernames are different (`@abakarovy` vs `abakarovyl`), so manual linking is required.

