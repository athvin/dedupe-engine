-- templated: __splink__df_representatives
-- physical: __splink__df_representatives_be2c56bd8
CREATE TABLE __splink__df_representatives_be2c56bd8 AS
select distinct
    node_id as node_id,
    node_id as representative,
    0 as stable
from __splink__df_nodes_with_composite_ids_5c2c29d44
