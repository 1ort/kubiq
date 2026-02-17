# Grammar

```ebnf
query         = where_clause select_clause?
where_clause  = "where" expr
select_clause = "select" path_list
path_list     = path (("," | ws+) path)*
expr          = condition (ws+ "and" ws+ condition)*
condition     = path ws* operator ws* value
operator      = "==" | "!="
path          = ident ("." ident)*
value         = quoted_string | bare_token
ident         = [A-Za-z_][A-Za-z0-9_-]*
```

Парсинг реализован на `nom`.
