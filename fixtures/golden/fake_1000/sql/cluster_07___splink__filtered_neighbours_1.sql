-- templated: __splink__filtered_neighbours_1
-- physical: __splink__filtered_neighbours_1_8e4cdd015
CREATE TABLE __splink__filtered_neighbours_1_8e4cdd015 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__df_neighbours_3c8e6b280 as n
left join __splink__representatives_1_0f2e058d2 as l
on l.node_id = n.node_id
left join __splink__representatives_1_0f2e058d2 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
