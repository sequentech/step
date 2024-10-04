

## Allow restricting Admins to specfic elections
From now on there is an option to restrict access Admin users to specific elections.
A new user attribute called permission_labels was added to the admin portal realm in Keycloak and it's multivalued. 
A new column was added to the election database. 
If there is no permission_label at the election everyone can access it.
If there if permission_label than the permission_labels from the x-hasura-permission_labels mapper from the user attribute needs to include the election permission label.
A new group was added to keycloak called admin-light and a new role and permission in Hasura called permission-label-write. which the new group does not have and can't edit the permission label at the election level and at the user level. 

### Important notes
1. A new user attribute and a new column was added to keycloak and Hasura. 
2. a new Mapper was added (custom Mapper to handle multivalued attribute for Hasura to read it right)
3. A new Permission was added and a new Group to keycloak called admin-light. 
