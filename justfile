set shell := ["bash", "-euo", "pipefail", "-c"]

bootstrap:
	@echo "Checking required tools..."
	@command -v git >/dev/null
	@command -v cargo >/dev/null
	@if command -v just >/dev/null; then \
		echo "just is installed"; \
	else \
		echo "just is not installed. Install: cargo install just"; \
	fi

verify:
	./scripts/verify.sh

automation-smoke:
	./scripts/automation-smoke.sh

hygiene-check:
	./scripts/hygiene-smoke.sh

docs-check:
	./scripts/docs-check.sh

verify-fast:
	cargo test -q

e2e:
	KUBIQ_E2E=1 cargo test --test e2e_minikube -- --nocapture

run *args:
	env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy cargo run -- {{args}}

feature NAME:
	NAME='{{NAME}}' ./scripts/git/feature.sh

ship MSG:
	MSG='{{MSG}}' ./scripts/git/ship.sh

push:
	./scripts/git/push.sh

pr-draft TYPE TITLE SCOPE="":
	TYPE='{{TYPE}}' TITLE='{{TITLE}}' SCOPE='{{SCOPE}}' ./scripts/pr/generate_pr.sh

sync-master BRANCH="":
	BRANCH='{{BRANCH}}' ./scripts/git/sync_master.sh
