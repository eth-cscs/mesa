# Inroduction

Mesa is a library to interact with Shasta CSM API.

The main goal of Mesa is to be an interface for applications dealing with Shasta CSM, an example of this may be [Manta](https://github.com/eth-cscs/manta). 

Mesa's main goal is security from memory safety, this is achieved by not using 'unsafe' code. In the future, we also want to provide good performance.

Potential users may want to try Mesa in the following scenarios:

 - Building applications to integrate Shasta systems based on CSM into their eco-system
 - Simplify CSM operations
 - Extend CSM functionalities

Mesa currently interacts with the following components:

 - HSM
 - CFS configuration
 - CFS session
 - CAPMC
 - BOS
 - BSS
 - IMS
 - Keycloak
 - K8s

# Deploy

```
cargo release patch --execute
```

 # Test

 ```
 cargo test -- --show-output
 ```
