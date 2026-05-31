#!/bin/sh
cat > /usr/share/nginx/html/config.js <<EOF
window.__PYLOS_ADMIN_KEY__ = "${PYLOS_ADMIN_KEY:-}";
EOF
exec "$@"
