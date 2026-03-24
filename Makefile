# 奇點世界 — 與 ./start 相同的「啟動前嚴格閘門」（非先跑再說，未通過不得建置上線）。
# 對齊 Rust/cargo 思維：靜態檢查 + 測試 + 資料契約，再交貨。

.PHONY: verify checkrooms build-server

verify: vet test checkrooms
	@echo "verify OK（vet + test + checkrooms -brackets -strict）"

vet:
	go vet ./...

test:
	go test ./... -count=1

checkrooms:
	mkdir -p bin
	go build -buildvcs=false -trimpath -o bin/checkrooms ./cmd/checkrooms
	./bin/checkrooms -brackets -strict

build-server:
	mkdir -p bin
	go build -buildvcs=false -trimpath -o bin/server .
