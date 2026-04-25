.PHONY: install clean reset dev-web dev-tauri dev-tauri-nvidia build-web build-tauri build-ui check lint format docker-build docker-up docker-down help

# ─── Setup ───────────────────────────────────────────────────────────

install: ## Install all workspace dependencies
	bun install

clean: ## Remove all build artifacts
	rm -rf apps/web/dist
	rm -rf apps/tauri/dist
	rm -rf packages/ui/dist
	rm -rf packages/ui/.svelte-kit

reset: clean ## Full reset: remove all node_modules and lockfiles, reinstall
	find . -name 'node_modules' -type d -prune -exec rm -rf {} +
	find . -name 'bun.lock' -not -path './bun.lock' -delete
	bun install

# ─── Development ─────────────────────────────────────────────────────

dev:
	@trap 'kill 0' EXIT INT TERM; \
	WEBKIT_DISABLE_DMABUF_RENDERER=1 bun run tauri dev

# ─── Build ───────────────────────────────────────────────────────────

build-desktop: ## Build the tauri app for production
	bun run tauri build

# ─── Quality ─────────────────────────────────────────────────────────

check: ## Run type checking across workspaces
	cd packages/ui && bun run check
	cd apps/tauri && bun run check

lint: ## Run linters
	cd packages/ui && bun run lint

format: ## Format code
	cd packages/ui && bun run format

# ─── Help ────────────────────────────────────────────────────────────

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-16s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
