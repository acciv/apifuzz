# apifuzz

Берёт OpenAPI-спецификацию, генерирует тестовые запросы и ищет нарушения контракта.

Работает только по объявленным схемам: корректные запросы, невалидные значения полей, перечисления и граничные строки. Никаких брутфорсов и обходов авторизации — только проверка своего API по его же спецификации.

## Зачем

Проверять API на тестовом стенде перед релизом: находить 500-е, коды, не описанные в спецификации, и слишком медленные ответы.

## Устройство

- **Rust** — разбирает OpenAPI JSON и генерирует тестовые случаи. C ABI и CLI `apifuzz`.
- **Go** — параллельно отправляет запросы и пишет результаты в JSONL.
- **Python** — анализирует результаты и печатает отчёт, умеет возвращать ненулевой код для CI.
- Docker, docker-compose, GitHub Actions.

Поддерживается подмножество OpenAPI 3 в JSON: `paths`, параметры `path`/`query`, `requestBody` с `application/json`, типы `string`, `integer`, `number`, `boolean`, `array`, `object`, `enum`, `required`.

## Что генерируется

На каждую операцию:

- базовый корректный запрос;
- для каждого параметра и поля — `null`, значение не того типа, пустая и длинная строка;
- все значения из `enum`;
- случай с удалённым обязательным полем.

## Запуск

```bash
make test
make demo
```

Демо поднимает тестовое API с двумя специальными ошибками, гоняет по нему сгенерированные запросы и показывает отчёт.

Вручную:

```bash
cargo build --release --manifest-path rust/Cargo.toml
./rust/target/release/apifuzz generate examples/openapi.json cases.json

cd go && go build -o /tmp/apifuzz-runner ./cmd/runner
/tmp/apifuzz-runner -spec examples/openapi.json -base-url http://127.0.0.1:9101 -out results.jsonl

PYTHONPATH=python python3 -m apifuzz --results results.jsonl --out report.json
```

Docker:

```bash
docker compose up --build
```

## Формат результата

Одна строка JSONL на запрос:

```json
{
  "index": 0,
  "name": "GET /users/{id} baseline",
  "method": "GET",
  "path": "/users/sample",
  "query": "page=1",
  "kind": "valid",
  "expected_status": [200, 400, 404],
  "status": 200,
  "response_body": "{\"id\":\"sample\"}",
  "duration_ms": 3.4
}
```

## Анализ

```bash
PYTHONPATH=python python3 -m apifuzz \
  --results results.jsonl --out report.json \
  --slow-ms 500 --fail-on high
```

Находки:

- `high` — сетевая ошибка, 5xx, базовый запрос не вернул 2xx;
- `medium` — код ответа не описан в спецификации;
- `low` — ответ медленнее `--slow-ms`.

Коды возврата: `0` — порог не превышен, `1` — превышен, `2` — ошибка чтения файла.

## Тесты

```bash
make test
```

Rust: 4 теста. Go: 3 теста. Python: 6 тестов.

Репозиторий: https://github.com/acciv/apifuzz

Сделал acciv.
