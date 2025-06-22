#!/bin/bash

# 1) Buildear
gnome-terminal -- bash -c "cargo build; exec bash"

# 2) Levantar el payment en una terminal
gnome-terminal -- bash -c "cargo run --bin payment; exec bash"

# 3) Levantar los servidores con pausas de 0.5 segundos
for port in 8080 8081 8082; do
  gnome-terminal -- bash -c "cargo run --bin server $port; exec bash"
  sleep 0.5
done

# 4) Levantar diez restaurantes en terminales diferentes
for i in {1..2}; do
  gnome-terminal -- bash -c "cargo run --bin restaurant resto$i; exec bash"
done

gnome-terminal -- bash -c "cargo run --bin restaurant resto_1; exec bash"

# 5) Levantar diez deliveries en terminales diferentes
for i in {1..5}; do
  gnome-terminal -- bash -c "cargo run --bin delivery delivery_$i; exec bash"
done

# 6) Levantar diez clientes en terminales diferentes
for i in {1..5}; do
  gnome-terminal -- bash -c "echo -e '1\npedido_$i\n' | cargo run --bin client client_$i; exec bash"
done