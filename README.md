# iam

## IAM
Iam is an api that allows to modify kratos identity.

It can be called with http or grpc call

### http

The http part of the api use mtls to secure the connection it is required to add
the needed files path to the config file.

To add data in an identity send a POST request with the uuid in the x-request-id
header to: ``/api/iam/policy``

To remove data in an identity send a DELETE request with the uuid in the x-request-id
header to: ``/api/iam/policy``

To replace data in an identity send a PUT request with the uuid in the x-request-id
header to: ``/api/iam/policy``

In all the preceding case you must use the payload json:
```json
{
    "id" = "string"
    "perm_type" = "string"
    "resource" = "string"
    "value" = "string"
    "mode" = integer
}
```
The id field represent the id of the identity to modify.

The perm_type field represent the type of permission to modify in wildcard case:
- user
- group
- organization

The ressource field represent the resource to modify.

the value field represent the data to modify the identity with

The mode field represent the type of data to edit:
- 0 = metadata_admin
- 1 = metadata_public
- 2 = traits


### grpc

the grpc part use the same model as the http but but use a different port (see
config file) 

For the method and payload format please refer to the .proto files in the /proto directory.
