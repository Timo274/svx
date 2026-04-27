# Build the Vite bundle in one stage, serve it with nginx in another.
# Deployed as a separate Fly.io app so the landing lives at svx.fly.dev
# next to the relay at svx-relay.fly.dev.

FROM node:22-alpine AS build
WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .
RUN npm run build

FROM nginx:1.27-alpine AS runtime

# Compact nginx config: listen on Fly's internal port, serve dist/, fall
# back to index.html for client-side routes, aggressively cache hashed
# assets, and expose a cheap /health endpoint for Fly's checker.
RUN rm /etc/nginx/conf.d/default.conf
COPY <<'NGINX' /etc/nginx/conf.d/svx.conf
server {
    listen       8080;
    server_name  _;
    root         /usr/share/nginx/html;
    index        index.html;

    gzip on;
    gzip_types text/plain text/css application/json application/javascript
               application/wasm image/svg+xml;

    location = /health {
        access_log off;
        add_header Content-Type application/json;
        return 200 '{"ok":true}';
    }

    location /assets/ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files $uri =404;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
NGINX

COPY --from=build /app/dist /usr/share/nginx/html

EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]
