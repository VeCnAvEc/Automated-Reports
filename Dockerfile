FROM rust:1.68.2 as builder

WORKDIR /usr/src/automated_reports

COPY . .

RUN cargo build --release

FROM debian:bullseye-slim
FROM debian

RUN apt-get update

RUN apt-get -y install wget
RUN apt -y install curl

# Устсановка wget для установки пакетов со стороних источников
RUN wget http://security.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.0g-2ubuntu4_amd64.deb
RUN dpkg -i libssl1.1_1.1.0g-2ubuntu4_amd64.deb

COPY --from=builder /usr/src/automated_reports/target/release/Automated_reports /usr/local/bin/Automated_reports

CMD ["Automated_reports"]