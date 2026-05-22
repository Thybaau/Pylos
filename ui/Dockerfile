FROM node:22-alpine AS builder

WORKDIR /app
COPY package*.json ./
RUN npm ci --silent
COPY . .
RUN npm run build

# ── Runtime : nginx pour servir le SPA ──────────────────────────────────────
FROM nginx:1.27 AS runtime

# Config nginx : SPA routing + proxy vers le backend Pylos
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
