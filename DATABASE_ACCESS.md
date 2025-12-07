# Database Access Guide

This guide explains how to access the SQLite database in your Docker container.

## Database Location

- **Inside container:** `/app/data/app.db`
- **On host system:** `./data/app.db` (relative to `docker-compose.yml` location)

The database is persisted via Docker volume mount: `./data:/app/data`

## Method 1: Access from Inside Container

### Connect to the running container:

```bash
docker exec -it business-assistant-backend bash
```

### Inside the container, use SQLite CLI:

```bash
# Connect to database
sqlite3 /app/data/app.db

# Or if sqlite3 is not installed in container, install it first:
apt-get update && apt-get install -y sqlite3
sqlite3 /app/data/app.db
```

### Useful SQLite commands:

```sql
-- List all tables
.tables

-- Show schema of a table
.schema users
.schema conversations
.schema telegram_users

-- Query data
SELECT * FROM users;
SELECT * FROM conversations;
SELECT * FROM telegram_users;

-- Exit SQLite
.quit
```

## Method 2: Access from Host System (Recommended)

Since the database is volume-mounted, you can access it directly from your host system:

### On Linux/Mac:

```bash
# Navigate to project directory
cd /path/to/alpha_backend

# Access database directly
sqlite3 ./data/app.db
```

### On Windows:

```powershell
# Navigate to project directory
cd C:\Users\User\Documents\GitHub\alpha_backend

# Use SQLite command line tool
sqlite3.exe .\data\app.db
```

## Method 3: Using SQLite GUI Tools

You can use any SQLite GUI tool to open the database file:

### Popular Options:

1. **DB Browser for SQLite** (Free, cross-platform)
   - Download: https://sqlitebrowser.org/
   - Open: `./data/app.db`

2. **DBeaver** (Free, cross-platform)
   - Download: https://dbeaver.io/
   - Create new SQLite connection
   - Path: `./data/app.db`

3. **VS Code Extension: SQLite Viewer**
   - Install extension in VS Code
   - Right-click `./data/app.db` → "Open Database"

## Method 4: Quick One-Liner Commands

### View all users:

```bash
docker exec business-assistant-backend sqlite3 /app/data/app.db "SELECT id, email, telegram_username FROM users;"
```

### Count conversations:

```bash
docker exec business-assistant-backend sqlite3 /app/data/app.db "SELECT COUNT(*) FROM conversations;"
```

### View recent conversations:

```bash
docker exec business-assistant-backend sqlite3 /app/data/app.db "SELECT id, user_id, title, created_at FROM conversations ORDER BY created_at DESC LIMIT 10;"
```

### View Telegram users:

```bash
docker exec business-assistant-backend sqlite3 /app/data/app.db "SELECT telegram_user_id, telegram_username, first_name, user_id FROM telegram_users;"
```

### Export database to SQL file:

```bash
docker exec business-assistant-backend sqlite3 /app/data/app.db .dump > database_backup.sql
```

### Backup database file:

```bash
# Copy from container to host
docker cp business-assistant-backend:/app/data/app.db ./app_backup.db

# Or directly from host (since it's volume-mounted)
cp ./data/app.db ./app_backup.db
```

## Common Queries

### Check user linking between platforms:

```sql
SELECT 
    u.id as main_user_id,
    u.email,
    u.telegram_username,
    tu.telegram_user_id,
    tu.first_name,
    tu.user_id as linked_user_id
FROM users u
LEFT JOIN telegram_users tu ON u.telegram_username = tu.telegram_username OR tu.user_id = u.id;
```

### View conversations with user info:

```sql
SELECT 
    c.id,
    c.title,
    u.email,
    u.telegram_username,
    c.created_at,
    (SELECT COUNT(*) FROM messages WHERE conversation_id = c.id) as message_count
FROM conversations c
LEFT JOIN users u ON c.user_id = u.id
ORDER BY c.created_at DESC;
```

### Find conversations by Telegram user:

```sql
SELECT 
    c.id,
    c.title,
    tu.telegram_user_id,
    tu.telegram_username,
    c.created_at
FROM conversations c
JOIN telegram_users tu ON c.user_id = tu.user_id
WHERE tu.telegram_user_id = 123456789;
```

## Troubleshooting

### Database file not found:

```bash
# Check if data directory exists
ls -la ./data/

# Check container volume mount
docker inspect business-assistant-backend | grep -A 10 Mounts

# Verify database file exists in container
docker exec business-assistant-backend ls -la /app/data/
```

### Permission issues:

```bash
# Fix permissions on host
chmod 664 ./data/app.db
chmod 755 ./data/

# Or check container permissions
docker exec business-assistant-backend ls -la /app/data/
```

### Database is locked:

This usually means the application is actively using the database. You can still read from it, but writes might fail. Consider:

1. Reading-only operations are safe
2. For writes, consider stopping the container temporarily:
   ```bash
   docker-compose stop backend
   # Do your database operations
   docker-compose start backend
   ```

## Security Note

⚠️ **Important:** Be careful when modifying the database directly. Always:
1. Create a backup first
2. Test changes on a copy
3. Understand the impact of your changes
4. Consider using migrations through the application when possible

