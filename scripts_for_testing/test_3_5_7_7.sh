#!/bin/bash

# Obtener la ruta absoluta a la raíz del repositorio (asumiendo que scripts está en la raíz/scripts)
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# 1) Buildear
gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo build; exec bash"

# 2) Levantar el payment en una terminal
gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo run --bin payment; exec bash"

# 3) Levantar los servidores con pausas de 0.5 segundos
for port in 8080 8081 8082; do
  gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo run --bin server $port; exec bash"
  sleep 0.5
done

# 4) Levantar cuatro restaurantes en terminales diferentes
for i in {1..5}; do
  gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo run --bin restaurant resto_$i; exec bash"
done



# 5) Levantar siete deliveries en terminales diferentes
for i in {1..7}; do
  gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo run --bin delivery delivery_$i; exec bash"
done

# 6) Levantar cinco clientes en terminales diferentes para el resto 1
for i in {1..5}; do
  gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && echo -e '1\npedido_$i\n' | cargo run --bin client client_$i; exec bash"
done

# 6) Levantar dos clientes en terminales diferentes para el resto 2
for i in {6..7}; do
  gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && echo -e '2\npedido_$i\n' | cargo run --bin client client_$i; exec bash"
done