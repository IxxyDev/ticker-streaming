# Quote Server and Client

## Сборка и тесты
- `cargo fmt`
- `cargo test`
- `cargo build`

## Запуск сервера
- `cargo run --bin server`  
  Слушает TCP `127.0.0.1:7878` и ждёт команд вида `STREAM udp://<ip>:<port> <T1,T2>`.

## Запуск клиента
- Подготовьте файл тикеров (по одному в строке), пример:
  ```
  AAPL
  TSLA
  ```
- Запустите:
  ```
  cargo run --bin client -- --server-addr 127.0.0.1:7878 --udp-host 127.0.0.1 --udp-port 34254 --tickers-file tickers.txt
  ```
- Флаги:
  - `--server-addr` — адрес TCP сервера.
  - `--udp-host` — адрес для UDP в команде STREAM.
  - `--udp-port` — порт для приёма UDP.
  - `--tickers-file` — путь к файлу тикеров.
- Клиент сам отправляет STREAM, принимает котировки, печатает их и каждые 2 секунды шлёт Ping.
- Логи включаются через `RUST_LOG=info` (по умолчанию `info`).

## Формат данных
- UDP-пакет: JSON `{"ticker":"AAPL","price":123.45,"volume":1000,"timestamp":1710000000000}`
- Ответ сервера на команду: `OK` или `ERR <причина>`.

## Keep-Alive
- Клиент отправляет `Ping` на адрес отправителя UDP.
- Сервер отвечает `Pong` и обновляет таймер активности.
- Если Ping не приходит ~5 секунд, поток клиента останавливается.
