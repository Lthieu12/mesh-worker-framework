## Améliorations du MWF

### 1. Ajouter une macro de dérivation pour les workers

Je souhaite disposer d’une macro permettant de simplifier la création d’un worker.

Actuellement, il faut écrire :

```rust
#[derive(Debug)]
pub struct FileSearch {}

#[async_trait]
impl mmf::Worker for FileSearch {
    // ...
}
```

L’objectif est de pouvoir écrire directement :

```rust
#[derive(Debug, Worker)]
pub struct FileSearch {}

impl FileSearch {
    // ...
}
```

La macro `Worker` devrait générer automatiquement l’implémentation nécessaire du trait `mmf::Worker`.

### 2. Accepter `&str` à la place de `&String`

Dans certaines fonctions, comme `send_message_and_wait`, `spawn_worker` ou `wait_worker_init`, je souhaite pouvoir passer directement une valeur de type `&str` au lieu d’une référence vers une `String`.

Par exemple, cette signature :

```rust
pub async fn wait_worker_init(
    worker_id: &String,
    timeout_ms: Option<u64>,
)
```

devrait devenir :

```rust
pub async fn wait_worker_init(
    worker_id: &str,
    timeout_ms: Option<u64>,
)
```
