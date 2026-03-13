//! Additional networking and infrastructure service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== NETWORKING / INFRASTRUCTURE ====================
        (
            "tpl-wireguard-easy",
            "WireGuard Easy",
            "The easiest way to run WireGuard VPN. Beautiful web UI for managing VPN clients.",
            "infrastructure",
            "wireguard",
            r#"services:
  wg-easy:
    image: ghcr.io/wg-easy/wg-easy:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-wg-easy}
    restart: unless-stopped
    ports:
      - "${WG_PORT:-51820}:51820/udp"
      - "${UI_PORT:-51821}:51821/tcp"
    environment:
      - WG_HOST=${WG_HOST:-change-me-to-your-public-ip}
      - PASSWORD=${PASSWORD:-changeme}
      - WG_PORT=${WG_PORT:-51820}
      - WG_DEFAULT_DNS=${DNS:-1.1.1.1}
      - WG_ALLOWED_IPS=${ALLOWED_IPS:-0.0.0.0/0,::/0}
      - UI_TRAFFIC_STATS=true
    cap_add:
      - NET_ADMIN
      - SYS_MODULE
    sysctls:
      - net.ipv4.conf.all.src_valid_mark=1
      - net.ipv4.ip_forward=1
    volumes:
      - wg_easy_data:/etc/wireguard
    labels:
      - "rivetr.managed=true"

volumes:
  wg_easy_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"wg-easy","secret":false},{"name":"WG_PORT","label":"WireGuard Port (UDP)","required":false,"default":"51820","secret":false},{"name":"UI_PORT","label":"Web UI Port","required":false,"default":"51821","secret":false},{"name":"WG_HOST","label":"Public IP / Hostname","required":true,"default":"","secret":false},{"name":"PASSWORD","label":"Web UI Password","required":true,"default":"","secret":true},{"name":"DNS","label":"DNS Server","required":false,"default":"1.1.1.1","secret":false},{"name":"ALLOWED_IPS","label":"Allowed IPs","required":false,"default":"0.0.0.0/0,::/0","secret":false}]"#,
        ),
        (
            "tpl-adguard-home",
            "AdGuard Home",
            "Network-wide ad and tracker blocking DNS server. Parental controls and per-client statistics.",
            "infrastructure",
            "adguard",
            r#"services:
  adguardhome:
    image: adguard/adguardhome:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-adguardhome}
    restart: unless-stopped
    ports:
      - "${DNS_PORT:-53}:53/tcp"
      - "${DNS_PORT:-53}:53/udp"
      - "${WEB_PORT:-3000}:3000/tcp"
      - "${HTTPS_PORT:-443}:443/tcp"
      - "${HTTPS_PORT:-443}:443/udp"
      - "${DOT_PORT:-853}:853/tcp"
    volumes:
      - adguard_work:/opt/adguardhome/work
      - adguard_conf:/opt/adguardhome/conf
    labels:
      - "rivetr.managed=true"

volumes:
  adguard_work:
  adguard_conf:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"adguardhome","secret":false},{"name":"DNS_PORT","label":"DNS Port","required":false,"default":"53","secret":false},{"name":"WEB_PORT","label":"Web UI Port","required":false,"default":"3000","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"DOT_PORT","label":"DNS-over-TLS Port","required":false,"default":"853","secret":false}]"#,
        ),
        (
            "tpl-cloudflared",
            "Cloudflare Tunnel",
            "Expose local services securely to the internet without opening firewall ports. Zero-trust tunnels.",
            "infrastructure",
            "cloudflare",
            r#"services:
  cloudflared:
    image: cloudflare/cloudflared:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-cloudflared}
    restart: unless-stopped
    command: tunnel --no-autoupdate run --token ${TUNNEL_TOKEN:-change-me}
    environment:
      - TUNNEL_TOKEN=${TUNNEL_TOKEN:-change-me}
    labels:
      - "rivetr.managed=true"
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"cloudflared","secret":false},{"name":"TUNNEL_TOKEN","label":"Cloudflare Tunnel Token","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-tailscale",
            "Tailscale",
            "Zero-config VPN mesh network. Connects your devices and services securely using WireGuard.",
            "infrastructure",
            "tailscale",
            r#"services:
  tailscale:
    image: tailscale/tailscale:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-tailscale}
    restart: unless-stopped
    hostname: ${HOSTNAME:-tailscale-container}
    cap_add:
      - NET_ADMIN
      - SYS_MODULE
    environment:
      - TS_AUTHKEY=${AUTH_KEY:-tskey-auth-change-me}
      - TS_STATE_DIR=/var/lib/tailscale
      - TS_USERSPACE=${USERSPACE:-false}
    volumes:
      - tailscale_data:/var/lib/tailscale
      - /dev/net/tun:/dev/net/tun
    labels:
      - "rivetr.managed=true"

volumes:
  tailscale_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"tailscale","secret":false},{"name":"AUTH_KEY","label":"Tailscale Auth Key","required":true,"default":"","secret":true},{"name":"HOSTNAME","label":"Node Hostname","required":false,"default":"tailscale-container","secret":false}]"#,
        ),
        (
            "tpl-headscale",
            "Headscale",
            "Open-source, self-hosted Tailscale control server. Run your own private WireGuard mesh network.",
            "infrastructure",
            "headscale",
            r#"services:
  headscale:
    image: headscale/headscale:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-headscale}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
      - "${GRPC_PORT:-50443}:50443"
    command: serve
    volumes:
      - headscale_config:/etc/headscale
      - headscale_data:/var/lib/headscale
    labels:
      - "rivetr.managed=true"

volumes:
  headscale_config:
  headscale_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"headscale","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"8080","secret":false},{"name":"GRPC_PORT","label":"gRPC Port","required":false,"default":"50443","secret":false}]"#,
        ),
        (
            "tpl-haproxy",
            "HAProxy",
            "Reliable, high-performance TCP/HTTP load balancer and proxy server for enterprise-grade deployments.",
            "infrastructure",
            "haproxy",
            r#"services:
  haproxy:
    image: haproxy:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-haproxy}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
      - "${STATS_PORT:-8404}:8404"
    volumes:
      - haproxy_config:/usr/local/etc/haproxy
    labels:
      - "rivetr.managed=true"

volumes:
  haproxy_config:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"haproxy","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"STATS_PORT","label":"Stats UI Port","required":false,"default":"8404","secret":false}]"#,
        ),
    ]
}
