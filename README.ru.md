# Альфа Будущее Хакатон, Серверная часть

Бэкенд‑сервис для бизнес‑ассистента, написанный на Rust с использованием Actix Web.

Сервис предоставляет аутентификацию, пользовательские профили, чат с историей переписки, простую бизнес‑аналитику, юридические страницы и возможность скачивания файлов.

---

## Возможности

- **Проверка работоспособности**
  - `GET /health`
  - Простой эндпоинт, чтобы убедиться, что сервер запущен.

- **Аутентификация и учетные записи пользователей**
  - `POST /api/auth/register`
    - Регистрирует нового пользователя по email, паролю, типу бизнеса и дополнительным полям профиля.
    - Создает начальную сессию и возвращает токен сессии.
  - `POST /api/auth/login`
    - Авторизует существующего пользователя по email и паролю.
    - Возвращает новый токен сессии при успешном входе.
  - `GET /api/auth/check-user?email={email}`
    - Проверяет, существует ли пользователь с указанным email.
  - `GET /api/auth/check-token?token={token}`
    - Проверяет, действителен ли токен сессии и не истек ли его срок.

- **Профиль пользователя**
  - `GET /api/auth/profile?token={token}`
    - Возвращает профиль аутентифицированного пользователя (без пароля), включая:
      - `id`, `email`, `business_type`, `created_at`
      - Дополнительно (опционально): `full_name`, `nickname`, `phone`, `country`, `gender`
  - `PUT /api/auth/profile?token={token}`
    - Обновляет поля профиля аутентифицированного пользователя:
      - `business_type`, `full_name`, `nickname`, `phone`, `country`, `gender`
    - Возвращает обновленный профиль.

- **Чат и диалоги**
  - `POST /api/chat/message`
    - Отправляет сообщение ассистенту и возвращает ответ.
    - Использует сохраненную историю диалогов, привязанную к `user_id`.
  - `GET /api/chat/conversations/{user_id}`
    - Возвращает список диалогов для указанного пользователя.
  - `GET /api/chat/history/{conversation_id}`
    - Возвращает историю сообщений для конкретного диалога.

- **Аналитика**
  - `GET /api/analytics/weekly-trends`
  - `POST /api/analytics/weekly-trends`
    - Получение или сохранение (upsert) трендов текущей недели:
      - Топ тренд (1-е место) с названием, процентом роста и процентом запросов
      - 2-е место с названием и процентом роста
      - Топ 3 географических тренда (страна и процент роста)
  - `GET /api/analytics/ai-analytics`
  - `POST /api/analytics/ai-analytics`
    - Получение или сохранение AI-аналитики:
      - Процент роста
      - Описание тренда
      - Массив данных уровня конкурентоспособности (минимум 5 значений для графика)
  - `GET /api/analytics/niches-month`
  - `POST /api/analytics/niches-month`
    - Получение или сохранение (upsert) ниш текущего месяца:
      - Массив ниш с названием и процентом изменения (положительный = рост, отрицательный = снижение)
  - `GET /api/analytics/top-trend`
  - `POST /api/analytics/top-trend`
    - Получение или сохранение (upsert) записи о «главном тренде» (legacy, для обратной совместимости).
  - `GET /api/analytics/popularity`
  - `POST /api/analytics/popularity`
    - Получение или сохранение (upsert) записей о трендах популярности (legacy, для обратной совместимости).

- **Файлы**
  - `GET /api/files/{id}`
    - Скачивание сохраненного файла по его ID.

- **Юридическая информация**
  - `GET /privacy-policy`
    - Возвращает содержимое страницы с политикой конфиденциальности.

---

## Технологический стек

- **Язык:** Rust
- **Веб‑фреймворк:** Actix Web
- **База данных:** SQLite (через `sqlx`)
- **Прочее:** `bcrypt` для хеширования паролей, `uuid` для идентификаторов, `chrono` для работы с временем, `dotenvy` для переменных окружения.

---

## Начало работы

### Предварительные требования

- Установленный Rust (стабильная версия)
- Установленный SQLite (или среда, где можно использовать файл SQLite)

### 1. Клонирование репозитория

```bash
git clone https://github.com/your-org/alpha_backend.git
cd alpha_backend
```

### 2. Настройка переменных окружения

Создайте файл `.env` в корне проекта (если он еще не существует):

```env
# Порт, на котором будет слушать сервер (опционально, по умолчанию: 3000)
PORT=3000

# URL базы данных SQLite
DATABASE_URL=sqlite://app.db
```

Если `DATABASE_URL` не задан, приложение по умолчанию использует `sqlite://app.db` в корне проекта.

### 3. Инициализация базы данных

При первом запуске приложение автоматически создаст и обновит структуру базы данных SQLite (таблицы `users`, `sessions`, `analytics_trends` и т.д.) с помощью SQL‑запросов в `db.rs`.

Обычно вам не нужно запускать миграции вручную; достаточно того, что процесс может записывать данные в файл `app.db`.

### 4. Запуск сервера

```bash
cargo run
```

По умолчанию сервер слушает по адресу:

```text
http://0.0.0.0:3000
```

Если указать переменную `PORT`, сервер будет слушать на соответствующем порту.

---

## Примеры запросов

### Регистрация пользователя

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

### Вход (логин)

```http
POST /api/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "strong-password"
}
```

В ответе будет возвращен `token`, который далее можно передавать как `?token=` при запросах к эндпоинтам профиля.

### Получение профиля

```http
GET /api/auth/profile?token=SESSION_TOKEN
```

### Обновление профиля

```http
PUT /api/auth/profile?token=SESSION_TOKEN
Content-Type: application/json

{
  "full_name": "New Name",
  "phone": "+123456789",
  "country": "US"
}
```

### Получение трендов недели

```http
GET /api/analytics/weekly-trends
```

### Сохранение трендов недели

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

### Получение AI-аналитики

```http
GET /api/analytics/ai-analytics
```

### Сохранение AI-аналитики

```http
POST /api/analytics/ai-analytics
Content-Type: application/json

{
  "increase": 10.0,
  "description": "Online education trend can be used to increase the brand as a source of benefit",
  "level_of_competitiveness": [25.5, 30.2, 35.8, 28.4, 32.1, 40.0, 38.7]
}
```

### Получение ниш месяца

```http
GET /api/analytics/niches-month
```

### Сохранение ниш месяца

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

---

## Заметки для разработки

- Сервер использует middleware `NormalizePath` для нормализации путей (убирает/добавляет завершающий слеш).
- CORS настроен максимально разрешающе, чтобы было проще интегрироваться с фронтендом.
- История диалогов хранится в памяти и привязана к `user_id`, в то время как данные пользователей и сессий сохраняются в SQLite.
