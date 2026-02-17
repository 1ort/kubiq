# Grammar

query      = resource where_clause select_clause?
where_clause = "where" expr
select_clause = "select" path_list
path_list  = path ("," path)*
expr       = condition ("and" condition)*
condition  = path operator value
operator   = "==" | "!="
path       = ident ("." ident)*
value      = string | number | boolean
