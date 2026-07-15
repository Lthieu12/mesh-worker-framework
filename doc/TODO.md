## Améliorations du MMF

### 1. Ajouter une macro de dérivation pour les modules

Je souhaite disposer d’une macro permettant de simplifier la création d’un module.

Actuellement, il faut écrire :

```rust
#[derive(Debug)]
pub struct FileSearch {}

#[async_trait]
impl mmf::Module for FileSearch {
    // ...
}
```

L’objectif est de pouvoir écrire directement :

```rust
#[derive(Debug, Module)]
pub struct FileSearch {}

impl FileSearch {
    // ...
}
```

La macro `Module` devrait générer automatiquement l’implémentation nécessaire du trait `mmf::Module`.

### 2. Accepter `&str` à la place de `&String`

Dans certaines fonctions, comme `send_message_and_wait`, `spawn_module` ou `wait_module_init`, je souhaite pouvoir passer directement une valeur de type `&str` au lieu d’une référence vers une `String`.

Par exemple, cette signature :

```rust
pub async fn wait_module_init(
    module_id: &String,
    timeout_ms: Option<u64>,
)
```

devrait devenir :

```rust
pub async fn wait_module_init(
    module_id: &str,
    timeout_ms: Option<u64>,
)
```
