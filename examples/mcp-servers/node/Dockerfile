# Étape 1 : Installer les dépendances
FROM node:20-alpine AS builder

WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm ci --only=production && npm cache clean --force

# Étape 2 : Image distroless Node.js
FROM gcr.io/distroless/nodejs20-debian12:nonroot

WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY server.js .

EXPOSE 8000

CMD ["server.js"]
