//! Database tooling and storage platform service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== DATABASES / STORAGE TOOLS ====================
        (
            "tpl-garage",
            "Garage",
            "Distributed S3-compatible object storage. Designed for geo-distributed deployments on bare metal.",
            "infrastructure",
            "garage",
            r#"services:
  garage:
    image: dxflrs/garage:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-garage}
    restart: unless-stopped
    ports:
      - "${S3_PORT:-3900}:3900"
      - "${RPC_PORT:-3901}:3901"
      - "${ADMIN_PORT:-3903}:3903"
    environment:
      - GARAGE_RPC_SECRET=${RPC_SECRET:-change-me-to-a-hex-string}
    volumes:
      - garage_meta:/var/lib/garage/meta
      - garage_data:/var/lib/garage/data
    labels:
      - "rivetr.managed=true"

volumes:
  garage_meta:
  garage_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"garage","secret":false},{"name":"S3_PORT","label":"S3 API Port","required":false,"default":"3900","secret":false},{"name":"RPC_PORT","label":"RPC Port","required":false,"default":"3901","secret":false},{"name":"ADMIN_PORT","label":"Admin Port","required":false,"default":"3903","secret":false},{"name":"RPC_SECRET","label":"RPC Secret (hex string)","required":true,"default":"","secret":true}]"#,
        ),
        (
            "tpl-seaweedfs",
            "SeaweedFS",
            "Simple and highly scalable distributed file system. Fast reads and writes for large volumes of files.",
            "infrastructure",
            "seaweedfs",
            r#"services:
  seaweedfs-master:
    image: chrislusf/seaweedfs:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-seaweedfs-master}
    restart: unless-stopped
    ports:
      - "${MASTER_PORT:-9333}:9333"
      - "${MASTER_GRPC_PORT:-19333}:19333"
    command: master -mdir=/data
    volumes:
      - seaweedfs_master_data:/data
    labels:
      - "rivetr.managed=true"

  seaweedfs-volume:
    image: chrislusf/seaweedfs:${VERSION:-latest}
    container_name: seaweedfs-volume
    restart: unless-stopped
    ports:
      - "${VOLUME_PORT:-8080}:8080"
    command: volume -mserver=seaweedfs-master:9333 -dir=/data -dataCenter=dc1
    volumes:
      - seaweedfs_volume_data:/data
    depends_on:
      - seaweedfs-master
    labels:
      - "rivetr.managed=true"

  seaweedfs-filer:
    image: chrislusf/seaweedfs:${VERSION:-latest}
    container_name: seaweedfs-filer
    restart: unless-stopped
    ports:
      - "${FILER_PORT:-8888}:8888"
      - "${S3_PORT:-8333}:8333"
    command: filer -master=seaweedfs-master:9333 -s3
    depends_on:
      - seaweedfs-master
    labels:
      - "rivetr.managed=true"

volumes:
  seaweedfs_master_data:
  seaweedfs_volume_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Master Container Name","required":false,"default":"seaweedfs-master","secret":false},{"name":"MASTER_PORT","label":"Master HTTP Port","required":false,"default":"9333","secret":false},{"name":"VOLUME_PORT","label":"Volume Port","required":false,"default":"8080","secret":false},{"name":"FILER_PORT","label":"Filer Port","required":false,"default":"8888","secret":false},{"name":"S3_PORT","label":"S3 API Port","required":false,"default":"8333","secret":false}]"#,
        ),
        (
            "tpl-nats",
            "NATS",
            "High-performance, cloud-native messaging system. Pub/sub, request-reply, and streaming.",
            "database",
            "nats",
            r#"services:
  nats:
    image: nats:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-nats}
    restart: unless-stopped
    ports:
      - "${CLIENT_PORT:-4222}:4222"
      - "${MONITOR_PORT:-8222}:8222"
      - "${ROUTING_PORT:-6222}:6222"
    command: -js -m 8222
    volumes:
      - nats_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  nats_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"nats","secret":false},{"name":"CLIENT_PORT","label":"Client Port","required":false,"default":"4222","secret":false},{"name":"MONITOR_PORT","label":"Monitor Port","required":false,"default":"8222","secret":false},{"name":"ROUTING_PORT","label":"Routing Port","required":false,"default":"6222","secret":false}]"#,
        ),
        (
            "tpl-kafka",
            "Apache Kafka",
            "Distributed event streaming platform. Handles trillions of events per day with high throughput.",
            "database",
            "kafka",
            r#"services:
  kafka:
    image: apache/kafka:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-kafka}
    restart: unless-stopped
    ports:
      - "${PORT:-9092}:9092"
    environment:
      - KAFKA_NODE_ID=1
      - KAFKA_PROCESS_ROLES=broker,controller
      - KAFKA_LISTENERS=PLAINTEXT://:9092,CONTROLLER://:9093
      - KAFKA_ADVERTISED_LISTENERS=PLAINTEXT://localhost:9092
      - KAFKA_CONTROLLER_QUORUM_VOTERS=1@localhost:9093
      - KAFKA_CONTROLLER_LISTENER_NAMES=CONTROLLER
      - KAFKA_INTER_BROKER_LISTENER_NAME=PLAINTEXT
      - KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR=1
      - KAFKA_GROUP_INITIAL_REBALANCE_DELAY_MS=0
      - KAFKA_AUTO_CREATE_TOPICS_ENABLE=true
    volumes:
      - kafka_data:/var/lib/kafka/data
    labels:
      - "rivetr.managed=true"

volumes:
  kafka_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"kafka","secret":false},{"name":"PORT","label":"Port","required":false,"default":"9092","secret":false}]"#,
        ),
        (
            "tpl-qdrant",
            "Qdrant",
            "High-performance vector database for AI applications. Stores and queries embeddings at scale.",
            "database",
            "qdrant",
            r#"services:
  qdrant:
    image: qdrant/qdrant:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-qdrant}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-6333}:6333"
      - "${GRPC_PORT:-6334}:6334"
    environment:
      - QDRANT__SERVICE__API_KEY=${API_KEY:-}
    volumes:
      - qdrant_data:/qdrant/storage
    labels:
      - "rivetr.managed=true"

volumes:
  qdrant_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"qdrant","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"6333","secret":false},{"name":"GRPC_PORT","label":"gRPC Port","required":false,"default":"6334","secret":false},{"name":"API_KEY","label":"API Key (optional)","required":false,"default":"","secret":true}]"#,
        ),
        (
            "tpl-weaviate",
            "Weaviate",
            "Open-source vector database with multi-modal support. Store, search, and retrieve embeddings.",
            "database",
            "weaviate",
            r#"services:
  weaviate:
    image: semitechnologies/weaviate:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-weaviate}
    restart: unless-stopped
    ports:
      - "${PORT:-8080}:8080"
      - "${GRPC_PORT:-50051}:50051"
    environment:
      - QUERY_DEFAULTS_LIMIT=25
      - AUTHENTICATION_ANONYMOUS_ACCESS_ENABLED=true
      - PERSISTENCE_DATA_PATH=/var/lib/weaviate
      - DEFAULT_VECTORIZER_MODULE=none
      - CLUSTER_HOSTNAME=node1
    volumes:
      - weaviate_data:/var/lib/weaviate
    labels:
      - "rivetr.managed=true"

volumes:
  weaviate_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"weaviate","secret":false},{"name":"PORT","label":"HTTP Port","required":false,"default":"8080","secret":false},{"name":"GRPC_PORT","label":"gRPC Port","required":false,"default":"50051","secret":false}]"#,
        ),
        (
            "tpl-valkey",
            "Valkey",
            "High-performance key-value store. Open-source Redis fork maintained by the Linux Foundation.",
            "database",
            "valkey",
            r#"services:
  valkey:
    image: valkey/valkey:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-valkey}
    restart: unless-stopped
    ports:
      - "${PORT:-6379}:6379"
    command: valkey-server --appendonly yes --requirepass ${PASSWORD:-}
    volumes:
      - valkey_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  valkey_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"valkey","secret":false},{"name":"PORT","label":"Port","required":false,"default":"6379","secret":false},{"name":"PASSWORD","label":"Password (optional)","required":false,"default":"","secret":true}]"#,
        ),
    ]
}
