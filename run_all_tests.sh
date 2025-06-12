set -e

# echo "Running tests for common..."
# cargo test -p common

# echo "Running tests for client..."
# cargo test -p client

# echo "Running tests for payment..."
# cargo test -p payment

# echo "Running tests for server..."
# cargo test -p server

echo "Running tests for delivery..."
cargo test -p delivery

# echo "Running tests for restaurant..."
# cargo test -p restaurant