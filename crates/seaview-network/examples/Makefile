# Makefile for seaview-network C++ example

# Compiler settings
CXX = g++
CXXFLAGS = -std=c++11 -Wall -Wextra -O2
LDFLAGS = -L../../../target/release -lseaview_network -lpthread -ldl -lm

# Include path for the generated header
INCLUDES = -I../include

# Target executable
TARGET = send_mesh

# Source files
SOURCES = send_mesh.cpp

# Build the example
$(TARGET): $(SOURCES)
	$(CXX) $(CXXFLAGS) $(INCLUDES) -o $@ $^ $(LDFLAGS)

# Build the Rust library first
.PHONY: rust-lib
rust-lib:
	cd ../../.. && cargo build --release -p seaview-network --features ffi

# Build everything
.PHONY: all
all: rust-lib $(TARGET)

# Run the example
.PHONY: run
run: all
	./$(TARGET)

# Clean build artifacts
.PHONY: clean
clean:
	rm -f $(TARGET)

# Help
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  all       - Build the Rust library and C++ example"
	@echo "  rust-lib  - Build only the Rust library"
	@echo "  $(TARGET) - Build only the C++ example"
	@echo "  run       - Build and run the example"
	@echo "  clean     - Remove build artifacts"
	@echo "  help      - Show this help message"
