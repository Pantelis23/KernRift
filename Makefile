# KernRift Self-Hosted Compiler
# Usage:
#   make              - build the compiler
#   make test         - run test suite
#   make install      - install to ~/.local/bin
#   make dist         - create distribution binaries for all platforms
#   make clean        - remove build artifacts
#   make bootstrap    - verify bootstrap (krc3 == krc4)

KERNRIFTC ?= kernriftc
INSTALL_DIR ?= $(HOME)/.local/bin
DIST_DIR = dist

SRCS = src/lexer.kr src/ast.kr src/parser.kr src/codegen.kr \
       src/codegen_aarch64.kr src/format_macho.kr src/format_pe.kr \
       src/format_archive.kr src/analysis.kr src/living.kr \
       src/runtime.kr src/main.kr

.PHONY: all build test install dist clean bootstrap

all: build

# Concatenate sources and compile with Rust kernriftc
build: build/krc build/krc2

build/krc.kr: $(SRCS)
	@mkdir -p build
	cat $(SRCS) > build/krc.kr

build/krc: build/krc.kr
	$(KERNRIFTC) --emit=hostexe build/krc.kr -o build/krc
	chmod +x build/krc

# Self-compile: krc → krc2 (Rust runtime always produces single-arch ELF)
build/krc2: build/krc build/krc.kr
	cp build/krc.kr test_input.kr
	./build/krc 2>/dev/null
	mv a.krbo build/krc2 2>/dev/null || mv a.out build/krc2 2>/dev/null
	chmod +x build/krc2
	rm -f test_input.kr a.krbo a.out

# Run test suite
test: build/krc2
	@echo "=== Running test suite ==="
	@echo '#!/bin/bash' > /tmp/krc-test && echo 'exec ./build/krc2 --arch=x86_64 "$$@"' >> /tmp/krc-test && chmod +x /tmp/krc-test
	@KRC=/tmp/krc-test bash tests/run_tests.sh || true

# Verify bootstrap convergence
bootstrap: build/krc2
	@echo "=== Bootstrap verification ==="
	@cp build/krc.kr /tmp/krc_bs_src.kr
	@./build/krc2 --arch=x86_64 /tmp/krc_bs_src.kr -o /tmp/krc3_bs 2>/dev/null
	@chmod +x /tmp/krc3_bs
	@/tmp/krc3_bs --arch=x86_64 /tmp/krc_bs_src.kr -o /tmp/krc4_bs 2>/dev/null
	@if diff /tmp/krc3_bs /tmp/krc4_bs >/dev/null 2>&1; then \
		echo "PASS: fixed point at $$(wc -c < /tmp/krc3_bs) bytes"; \
	else \
		echo "FAIL: krc3 != krc4"; exit 1; \
	fi
	@rm -f /tmp/krc_bs_src.kr /tmp/krc3_bs /tmp/krc4_bs

# Install to INSTALL_DIR
install: build/krc2
	@mkdir -p $(INSTALL_DIR)
	cp build/krc2 $(INSTALL_DIR)/krc
	chmod +x $(INSTALL_DIR)/krc
	@echo "Installed: $(INSTALL_DIR)/krc"
	@echo "Ensure $(INSTALL_DIR) is in your PATH"

# Create distribution binaries
dist: build/krc2
	@mkdir -p $(DIST_DIR)
	@echo "=== Building distribution ==="
	@# x86_64 Linux ELF
	cp build/krc2 $(DIST_DIR)/krc-linux-x86_64
	chmod +x $(DIST_DIR)/krc-linux-x86_64
	@echo "  krc-linux-x86_64"
	@# ARM64 Linux ELF (cross-compiled)
	./build/krc2 --arch=arm64 build/krc.kr -o $(DIST_DIR)/krc-linux-arm64 2>/dev/null
	chmod +x $(DIST_DIR)/krc-linux-arm64
	@echo "  krc-linux-arm64"
	@# Fat binary (default)
	./build/krc2 build/krc.kr -o $(DIST_DIR)/krc.krbo 2>/dev/null
	@echo "  krc.krbo (x86_64 + arm64)"
	@# Source distribution
	cp build/krc.kr $(DIST_DIR)/krc-source.kr
	@echo "  krc-source.kr"
	@echo ""
	@ls -la $(DIST_DIR)/
	@echo ""
	@echo "=== Distribution complete ==="

# Clean all build artifacts
clean:
	rm -rf build/krc build/krc2 build/krc.kr
	rm -rf $(DIST_DIR)
	rm -f a.out output.elf test_input.kr
	rm -f krc2 krc3 krc4 krc_arm64
	rm -f *.elf *.out
	@echo "Cleaned."
