## Améliorations du MWF

### 1. Ajouter une macro de dérivation pour les workers

Je souhaite disposer d’une macro permettant de simplifier la création d’un worker.

Actuellement, il faut écrire :

```rust
#[derive(Debug)]
pub struct FileSearch {}

#[async_trait]
impl mwf::Worker for FileSearch {
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

La macro `Worker` devrait générer automatiquement l’implémentation nécessaire du trait `mwf::Worker`.
