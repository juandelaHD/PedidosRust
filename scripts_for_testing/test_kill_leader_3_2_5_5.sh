#!/bin/bash

# Obtener la ruta absoluta a la raíz del repositorio (asumiendo que scripts está en la raíz/scripts)
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# 1) Buildear
gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo build; exec bash"

# 2) Levantar el payment en una terminal
gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo run --bin payment; exec bash"

# 3) Levantar los servidores con pausas de 0.5 segundos y luego de 4 segundos matar al lider
for port in 8080 8081 8082; do
  gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo run --bin server $port; exec bash"
  sleep 0.5
done



# 4) Levantar dos restaurantes en terminales diferentes
for i in {1..2}; do
  gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo run --bin restaurant resto_$i; exec bash"
done



# 5) Levantar cinco deliveries en terminales diferentes
for i in {1..5}; do
  gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && cargo run --bin delivery delivery_$i; exec bash"
done

# 6) Levantar cinco clientes en terminales diferentes
for i in {1..5}; do
  gnome-terminal -- bash -c "cd \"$REPO_ROOT\" && echo -e '1\npedido_$i\n' | cargo run --bin client client_$i; exec bash"
done


# 7) Esperar 2 segundos antes de finalizar
sleep 3
pkill -SIGINT -f "server 8080"
