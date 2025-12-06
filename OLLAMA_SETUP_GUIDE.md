# Руководство по установке GPT-OSS 20B на Ubuntu с использованием Ollama

## Введение

### Что такое Ollama?

Ollama - это инструмент для локального запуска больших языковых моделей (LLM). Он позволяет запускать модели на вашем собственном сервере без необходимости подключаться к облачным API.

### Что такое GPT-OSS 20B?

GPT-OSS 20B - это открытая языковая модель с 20 миллиардами параметров, которая может использоваться коммерчески. Это альтернатива проприетарным моделям, таким как GPT-4, но работает полностью локально.

### Преимущества локального развертывания

- **Конфиденциальность**: Все данные остаются на вашем сервере
- **Нет ограничений на использование**: Неограниченное количество запросов
- **Предсказуемые расходы**: Нет платы за API-вызовы
- **Полный контроль**: Вы контролируете производительность и настройки
- **Работа в автономном режиме**: Не требуется интернет-соединение после установки

---

## Предварительные требования

### Системные требования

Для работы GPT-OSS 20B рекомендуется:

- **RAM**: Минимум 24 GB (рекомендуется 32 GB или больше)
- **CPU**: Многоядерный процессор (рекомендуется 8+ ядер)
- **Дисковое пространство**: 
  - Минимум 40 GB свободного места для модели
  - Дополнительно 10-20 GB для системы и кэша
- **GPU** (опционально, но значительно ускоряет работу):
  - NVIDIA GPU с минимум 16 GB VRAM (рекомендуется 24 GB+)
  - Поддержка CUDA 11.8 или выше

### Версия Ubuntu

- **Ubuntu 20.04 LTS** или новее (рекомендуется Ubuntu 22.04 LTS или 24.04 LTS)

### Необходимые пакеты

Перед установкой убедитесь, что у вас установлены:

```bash
sudo apt update
sudo apt install -y curl wget git
```

Если вы планируете использовать GPU:

```bash
# Проверка наличия NVIDIA GPU
nvidia-smi

# Если команда не найдена, установите драйверы NVIDIA
sudo apt install -y nvidia-driver-535  # или более новая версия
```

---

## Установка Ollama на Ubuntu

### Способ 1: Официальная установка через скрипт (рекомендуется)

Это самый простой и рекомендуемый способ установки Ollama:

```bash
# Скачайте и запустите установочный скрипт
curl -fsSL https://ollama.com/install.sh | sh
```

Скрипт автоматически:
- Добавит репозиторий Ollama
- Установит Ollama и необходимые зависимости
- Создаст системный сервис для автоматического запуска
- Настроит права доступа

### Способ 2: Установка через Docker (альтернативный)

Если вы предпочитаете использовать Docker:

```bash
# Установите Docker (если еще не установлен)
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Добавьте текущего пользователя в группу docker
sudo usermod -aG docker $USER

# Выйдите и войдите снова, чтобы изменения вступили в силу

# Запустите Ollama в Docker
docker run -d -v ollama:/root/.ollama -p 11434:11434 --name ollama ollama/ollama

# Проверьте статус
docker ps | grep ollama
```

### Проверка установки

После установки проверьте, что Ollama работает:

```bash
# Проверьте версию Ollama
ollama --version

# Проверьте статус сервиса (для системной установки)
sudo systemctl status ollama

# Или проверьте Docker контейнер
docker ps | grep ollama
```

Если Ollama установлен правильно, вы должны увидеть информацию о версии и работающий сервис.

---

## Загрузка GPT-OSS 20B модели

### Определение точного имени модели

GPT-OSS 20B может быть доступен под разными именами в Ollama. Проверьте доступные варианты:

```bash
# Поиск моделей GPT-OSS
ollama list  # Покажет установленные модели

# Поиск в библиотеке моделей (может потребоваться браузер)
# Откройте https://ollama.com/library и найдите gpt-oss
```

Возможные варианты названий:
- `gpt-oss-20b`
- `gpt-oss:20b`
- `gpt-oss/20b`

### Скачивание модели

После определения точного названия скачайте модель:

```bash
# Пример команды (замените на актуальное название модели)
ollama pull gpt-oss:20b
```

Процесс загрузки может занять значительное время в зависимости от скорости интернета (модель весит примерно 40 GB).

### Проверка размера и места на диске

```bash
# Проверьте установленные модели
ollama list

# Проверьте использование диска
du -sh ~/.ollama/models/  # для системной установки
# или
docker exec ollama du -sh /root/.ollama/models/  # для Docker

# Проверьте свободное место
df -h
```

Убедитесь, что у вас достаточно свободного места на диске.

### Альтернативные варианты моделей

Если GPT-OSS 20B недоступен, рассмотрите похожие открытые модели:

```bash
# Llama 3.1 70B (требует больше ресурсов)
ollama pull llama3.1:70b

# Mistral 7B (легче, но меньше параметров)
ollama pull mistral:7b

# Qwen 2.5 32B (хорошая альтернатива)
ollama pull qwen2.5:32b
```

---

## Настройка Ollama

### Настройка переменных окружения

Для системной установки создайте или отредактируйте файл конфигурации:

```bash
# Создайте файл конфигурации
sudo nano /etc/systemd/system/ollama.service.d/override.conf
```

Добавьте следующие переменные окружения:

```ini
[Service]
Environment="OLLAMA_HOST=0.0.0.0:11434"
Environment="OLLAMA_NUM_PARALLEL=2"
Environment="OLLAMA_MAX_LOADED_MODELS=1"
Environment="OLLAMA_NUM_GPU=1"  # Если используете GPU
Environment="OLLAMA_KEEP_ALIVE=24h"
```

Перезагрузите сервис:

```bash
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

Для Docker используйте переменные окружения в docker-compose или docker run:

```bash
docker run -d \
  -v ollama:/root/.ollama \
  -p 11434:11434 \
  -e OLLAMA_HOST=0.0.0.0:11434 \
  -e OLLAMA_NUM_PARALLEL=2 \
  -e OLLAMA_MAX_LOADED_MODELS=1 \
  --name ollama \
  ollama/ollama
```

### Конфигурация для максимальной производительности

#### Использование GPU (рекомендуется)

Если у вас есть NVIDIA GPU, убедитесь, что Ollama его использует:

```bash
# Проверьте, видит ли Ollama GPU
ollama ps

# Или через API
curl http://localhost:11434/api/ps
```

Для принудительного использования GPU:

```bash
# Установите переменную окружения
export OLLAMA_NUM_GPU=1

# Или в systemd override
Environment="OLLAMA_NUM_GPU=1"
```

#### Настройка памяти

Для оптимизации использования памяти:

```bash
# Ограничьте количество параллельных запросов
export OLLAMA_NUM_PARALLEL=1

# Ограничьте количество загруженных моделей
export OLLAMA_MAX_LOADED_MODELS=1

# Настройте время хранения модели в памяти
export OLLAMA_KEEP_ALIVE=24h
```

#### Оптимизация для CPU

Если у вас нет GPU:

```bash
# Установите количество потоков (обычно количество ядер CPU)
export OLLAMA_NUM_THREAD=8

# Используйте quantization для уменьшения размера модели в памяти
# Попробуйте загрузить quantized версию модели
ollama pull gpt-oss:20b-q4_0  # если доступна
```

### Настройка сети и портов

По умолчанию Ollama слушает на `127.0.0.1:11434`. Для доступа из других контейнеров или серверов:

```bash
# Измените хост на 0.0.0.0 для прослушивания на всех интерфейсах
export OLLAMA_HOST=0.0.0.0:11434

# Или только на внутреннем интерфейсе
export OLLAMA_HOST=172.17.0.1:11434
```

Перезапустите Ollama после изменения настроек:

```bash
# Для системной установки
sudo systemctl restart ollama

# Для Docker
docker restart ollama
```

---

## Интеграция с бэкендом

### Обновление docker-compose.yml

Добавьте сервис Ollama в ваш `docker-compose.yml`:

```yaml
version: '3.8'

services:
  ollama:
    image: ollama/ollama:latest
    container_name: ollama
    ports:
      - "11434:11434"
    volumes:
      - ollama-data:/root/.ollama
    environment:
      - OLLAMA_HOST=0.0.0.0:11434
      - OLLAMA_NUM_PARALLEL=2
      - OLLAMA_MAX_LOADED_MODELS=1
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:11434/api/tags"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]  # Только если есть GPU

  backend:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: business-assistant-backend
    ports:
      - "8080:8080"
    environment:
      - PORT=8080
      - DATABASE_URL=sqlite:///app/data/app.db
      - RUST_LOG=info
      # AI Provider Configuration
      - AI_PROVIDER=ollama  # Используйте 'openrouter' для облачного API
      - OLLAMA_BASE_URL=http://ollama:11434
      - OLLAMA_MODEL=gpt-oss:20b
      # OpenRouter (для fallback или переключения)
      - OPENROUTER_API_KEY=${OPENROUTER_API_KEY:-}
      - OPENROUTER_MODEL=${OPENROUTER_MODEL:-openrouter/auto}
      # ... остальные переменные
    volumes:
      - ./data:/app/data
      - ./assets:/app/assets:ro
    depends_on:
      ollama:
        condition: service_healthy
    restart: unless-stopped

volumes:
  ollama-data:
    driver: local
```

### Настройка переменных окружения бэкенда

В файле `.env` (или через переменные окружения Docker) добавьте:

```bash
# AI Provider: 'ollama' или 'openrouter'
AI_PROVIDER=ollama

# Ollama Configuration
OLLAMA_BASE_URL=http://ollama:11434
OLLAMA_MODEL=gpt-oss:20b

# OpenRouter (опционально, для fallback)
OPENROUTER_API_KEY=your_key_here
OPENROUTER_MODEL=openrouter/auto
```

### Проверка подключения

Проверьте, что бэкенд может подключиться к Ollama:

```bash
# Из контейнера бэкенда
docker exec -it business-assistant-backend curl http://ollama:11434/api/tags

# Или с хоста
curl http://localhost:11434/api/tags
```

Ожидаемый ответ должен содержать список установленных моделей.

---

## Тестирование

### Проверка работы Ollama API

#### 1. Проверка доступности API

```bash
# Проверка статуса
curl http://localhost:11434/api/tags

# Должен вернуть список моделей в JSON формате
```

#### 2. Тестовый запрос через curl

Создайте тестовый запрос:

```bash
curl http://localhost:11434/api/chat -d '{
  "model": "gpt-oss:20b",
  "messages": [
    {
      "role": "user",
      "content": "Привет! Расскажи о себе."
    }
  ],
  "stream": false
}'
```

Ожидаемый ответ:

```json
{
  "model": "gpt-oss:20b",
  "created_at": "2024-01-01T00:00:00.000Z",
  "message": {
    "role": "assistant",
    "content": "Привет! Я GPT-OSS 20B..."
  },
  "done": true
}
```

#### 3. Тест с streaming (опционально)

```bash
curl http://localhost:11434/api/chat -d '{
  "model": "gpt-oss:20b",
  "messages": [
    {
      "role": "user",
      "content": "Что такое машинное обучение?"
    }
  ],
  "stream": true
}'
```

### Проверка работы через бэкенд

#### 1. Проверка health check

```bash
curl http://localhost:8080/health
```

#### 2. Тестовый запрос к API чата

```bash
curl -X POST http://localhost:8080/api/chat/message \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test-user-id",
    "message": "Привет! Как дела?",
    "category": "general",
    "business_type": "ecommerce"
  }'
```

#### 3. Проверка логов

```bash
# Логи бэкенда
docker logs business-assistant-backend -f

# Логи Ollama
docker logs ollama -f

# Или для системной установки
sudo journalctl -u ollama -f
```

---

## Оптимизация и устранение проблем

### Оптимизация использования памяти

#### 1. Мониторинг использования ресурсов

```bash
# Проверка использования памяти
docker stats ollama

# Или для системной установки
top -p $(pgrep ollama)
htop
```

#### 2. Настройка keep_alive

Уменьшите время хранения модели в памяти, если редко используется:

```bash
export OLLAMA_KEEP_ALIVE=5m  # Вместо 24h
```

#### 3. Использование quantized моделей

Используйте quantized версии моделей для экономии памяти:

```bash
# Загрузите quantized версию (если доступна)
ollama pull gpt-oss:20b-q4_0
```

### Troubleshooting типичных проблем

#### Проблема: Ollama не запускается

**Решение:**
```bash
# Проверьте логи
sudo journalctl -u ollama -n 50

# Проверьте порт
sudo netstat -tulpn | grep 11434

# Проверьте права доступа
sudo chown -R ollama:ollama ~/.ollama
```

#### Проблема: Недостаточно памяти

**Решение:**
```bash
# Уменьшите OLLAMA_NUM_PARALLEL
export OLLAMA_NUM_PARALLEL=1

# Используйте swap (временное решение)
sudo fallocate -l 8G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

#### Проблема: Медленная генерация ответов

**Решения:**
- Убедитесь, что используется GPU
- Увеличьте количество ядер CPU
- Используйте более быстрый диск (SSD вместо HDD)
- Рассмотрите использование более легкой модели

#### Проблема: Модель не загружается

**Решение:**
```bash
# Проверьте доступное место
df -h

# Проверьте целостность модели
ollama list
ollama show gpt-oss:20b

# Попробуйте перезагрузить модель
ollama rm gpt-oss:20b
ollama pull gpt-oss:20b
```

#### Проблема: Бэкенд не может подключиться к Ollama

**Решение:**
```bash
# Проверьте сеть Docker
docker network inspect bridge

# Проверьте, что сервисы в одной сети
docker-compose ps

# Проверьте firewall
sudo ufw status
sudo ufw allow 11434/tcp
```

### Мониторинг производительности

#### Создайте скрипт мониторинга

Создайте файл `monitor_ollama.sh`:

```bash
#!/bin/bash

echo "=== Ollama Status ==="
ollama ps

echo ""
echo "=== Memory Usage ==="
docker stats ollama --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}"

echo ""
echo "=== Model Size ==="
du -sh ~/.ollama/models/

echo ""
echo "=== API Test ==="
curl -s http://localhost:11434/api/tags | jq '.models[] | {name: .name, size: .size}'
```

Сделайте скрипт исполняемым:

```bash
chmod +x monitor_ollama.sh
./monitor_ollama.sh
```

---

## Безопасность

### Рекомендации по безопасности

#### 1. Ограничьте доступ к Ollama API

Если Ollama не должен быть доступен извне:

```bash
# Настройте Ollama слушать только на localhost
export OLLAMA_HOST=127.0.0.1:11434

# Или на внутреннем Docker network
export OLLAMA_HOST=0.0.0.0:11434  # Внутри Docker сети
```

#### 2. Настройка firewall

```bash
# Разрешите доступ только с локальной сети
sudo ufw allow from 172.17.0.0/16 to any port 11434

# Или полностью заблокируйте внешний доступ
sudo ufw deny 11434/tcp
```

#### 3. Использование reverse proxy с аутентификацией

Настройте Nginx как reverse proxy с базовой аутентификацией:

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location /api/ {
        proxy_pass http://localhost:11434/api/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        
        # Базовая аутентификация
        auth_basic "Restricted";
        auth_basic_user_file /etc/nginx/.htpasswd;
    }
}
```

#### 4. Регулярные обновления

```bash
# Обновите Ollama
sudo systemctl stop ollama
curl -fsSL https://ollama.com/install.sh | sh
sudo systemctl start ollama

# Или для Docker
docker pull ollama/ollama:latest
docker-compose up -d ollama
```

---

## Резервное копирование и обновление

### Как обновить модель

#### 1. Обновление Ollama

```bash
# Для системной установки
sudo systemctl stop ollama
curl -fsSL https://ollama.com/install.sh | sh
sudo systemctl start ollama

# Для Docker
docker-compose pull ollama
docker-compose up -d ollama
```

#### 2. Обновление модели

```bash
# Удалите старую версию
ollama rm gpt-oss:20b

# Загрузите новую версию
ollama pull gpt-oss:20b

# Проверьте версию
ollama show gpt-oss:20b
```

### Резервное копирование данных

#### 1. Резервное копирование моделей

```bash
# Создайте резервную копию директории с моделями
tar -czf ollama-models-backup-$(date +%Y%m%d).tar.gz ~/.ollama/models/

# Или для Docker
docker run --rm -v ollama:/data -v $(pwd):/backup ubuntu tar czf /backup/ollama-backup.tar.gz /data
```

#### 2. Автоматическое резервное копирование

Создайте скрипт `backup_ollama.sh`:

```bash
#!/bin/bash

BACKUP_DIR="/path/to/backups"
DATE=$(date +%Y%m%d_%H%M%S)

# Создайте директорию для бэкапов
mkdir -p $BACKUP_DIR

# Резервная копия моделей
tar -czf $BACKUP_DIR/ollama-models-$DATE.tar.gz ~/.ollama/models/

# Удалите старые бэкапы (старше 7 дней)
find $BACKUP_DIR -name "ollama-models-*.tar.gz" -mtime +7 -delete

echo "Backup completed: ollama-models-$DATE.tar.gz"
```

Добавьте в crontab:

```bash
# Резервное копирование каждый день в 2:00
0 2 * * * /path/to/backup_ollama.sh >> /var/log/ollama-backup.log 2>&1
```

#### 3. Восстановление из резервной копии

```bash
# Остановите Ollama
sudo systemctl stop ollama

# Распакуйте резервную копию
tar -xzf ollama-models-backup-YYYYMMDD.tar.gz -C ~/

# Или для Docker
docker run --rm -v ollama:/data -v $(pwd):/backup ubuntu tar xzf /backup/ollama-backup.tar.gz -C /

# Запустите Ollama
sudo systemctl start ollama
```

---

## Дополнительные ресурсы

### Полезные команды

```bash
# Список всех моделей
ollama list

# Информация о модели
ollama show gpt-oss:20b

# Запуск интерактивного чата
ollama run gpt-oss:20b

# Удаление модели
ollama rm gpt-oss:20b

# Копирование модели
ollama cp gpt-oss:20b gpt-oss:20b-backup
```

### Полезные ссылки

- [Официальная документация Ollama](https://github.com/ollama/ollama/blob/main/docs/README.md)
- [Библиотека моделей Ollama](https://ollama.com/library)
- [API документация Ollama](https://github.com/ollama/ollama/blob/main/docs/api.md)

### Поддержка

Если у вас возникли проблемы:

1. Проверьте логи Ollama
2. Проверьте системные ресурсы
3. Изучите документацию Ollama
4. Создайте issue на GitHub Ollama

---

## Заключение

После выполнения всех шагов этого руководства у вас должен быть полностью функциональный локальный сервер с GPT-OSS 20B, интегрированный с вашим бэкендом. Локальное развертывание обеспечивает конфиденциальность, предсказуемость затрат и полный контроль над системой.

Удачи с вашим проектом!

