# Chat NP1

Por falta de tempo e experiência, não foi feita interface gráfica. Além disso, o trabalho está bem mal acabado e longe de ser código de que eu me orgulho. É bem fácil causar crashes (ou panics), mas as funcionalidades básicas estão implementadas. 

Os seguintes opcionais foram implementados:
- Utilizar o formato binário no protocolo de comunicação entre cliente e servidor (ver src/message.rs)

- Permitir que o usuário possa criar uma nova sala de bate papo pública, tornando-se o administrador dela onde possa retirar uma pessoa da mesma. (ver src/chatserver.rs)

## Quirks:
- Direitos de administrador são dados por ordem de chegada. O primeiro a entrar numa sala é considerado administrador. Ao sair, o segundo é considerado administrador, e assim em diante.

## Instalando e Executando
Requer a linguagem [Rust](https://www.rustup.rs/) instalada.

Para compilar, ir para root do repositório (onde tem o arquivo `Cargo.toml`) e executar:
```
cargo build
```

Para iniciar o servidor, executar:
```
cargo run --bin server
```

Para iniciar o cliente, é necessário ir para o executável diretamente:
```
cd target/debug
./client [USERNAME]
```
Onde [USERNAME] é o nome com que se conectar com o servidor. Isso é necessário para poder abrir múltiplos clientes de uma vez (rodar pelo cargo faz recompilação, sendo bloquado se o executável está sendo usado).
