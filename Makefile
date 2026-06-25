export DATABASE_URL := postgres://budgete:budgete@127.0.0.1:5432/budgete
export LOCAL_MODE := true

WEB_DIR := packages/web

.PHONY: dev serve db-start db-stop install-tools check-tools

dev: check-tools db-start serve

serve:
	cd $(WEB_DIR) && dx serve

db-start:
	@(echo > /dev/tcp/127.0.0.1/5432) 2>/dev/null || (docker compose up -d && for i in {1..10}; do (echo > /dev/tcp/127.0.0.1/5432) 2>/dev/null && sleep 1 && exit 0 || sleep 1; done; exit 1)

db-stop:
	docker compose down

install-tools:
	cargo install sqlx-cli --no-default-features --features postgres
	cargo install dioxus-cli --version 0.7.9

check-tools:
	@command -v sqlx >/dev/null 2>&1 || { echo "error: run 'make install-tools'"; exit 1; }
	@command -v dx >/dev/null 2>&1 || { echo "error: run 'make install-tools'"; exit 1; }
