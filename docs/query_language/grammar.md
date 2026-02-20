# Grammar

```ebnf
query         = where_clause suffix_clause*
where_clause  = "where" expr
suffix_clause = select_clause | order_clause
select_clause = "select" path_list
order_clause  = "order" ws+ "by" ws+ order_key_list
path_list     = path (("," | ws+) path)*
order_key_list = order_key ("," order_key)*
order_key     = path (ws+ direction)?
direction     = "asc" | "desc"
expr          = condition (ws+ "and" ws+ condition)*
condition     = path ws* operator ws* value
operator      = "==" | "!="
path          = ident ("." ident)*
value         = quoted_string | bare_token
ident         = [A-Za-z_][A-Za-z0-9_-]*
```

Ограничения:

- `select` и `order by` можно использовать в любом порядке после `where`.
- Каждый из clause (`select`, `order by`) может встречаться не более одного раза.

Парсинг реализован на `nom`.
