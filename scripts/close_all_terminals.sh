#!/bin/bash

echo "Closing all terminal windows..."

# Kill de todos los procesos gnome-terminal (requiere permisos para matar procesos propios)
pkill gnome-terminal

echo "All terminal windows have been closed."