# Framework relationships

Express ESM default imports and named Router imports can establish const factory
receivers (express(), express.Router(), Router()). Literal HTTP-method registrations
create route nodes and ROUTES_TO relationships to resolvable handler identifiers.
Inline handler calls belong to the route. The displayed path is the declaration;
mount prefixes, runtime URLs, chained route builders, receiver mutation/aliases,
CommonJS factories and computed paths are not inferred. A random object.get()
does not establish a route. Parameter-shadowed receivers are excluded.

Uppercase JSX tags create REFERENCES from their owning function to a local or
imported declaration when the shared resolver can resolve it. Lowercase HTML tags
are excluded. Namespaces use static import bindings. Higher-order components,
render props, dynamic JSX factories and runtime providers are not inferred.

All relationships retain the AST span and registration/JSX evidence. These narrow
adapters are grounded in the Express routing and React component syntax:
https://expressjs.com/en/guide/routing/
https://react.dev/learn/your-first-component
