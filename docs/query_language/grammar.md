# Grammar

query      = resource where_clause?
where_clause = "where" expr
expr       = condition ("and" condition)*
condition  = path operator value
operator   = "==" | "!="
path       = ident ("." ident)*
value      = string | number | boolean
