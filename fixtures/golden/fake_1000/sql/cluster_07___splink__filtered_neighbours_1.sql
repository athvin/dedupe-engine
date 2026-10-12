-- templated: __splink__filtered_neighbours_1
-- physical: __splink__filtered_neighbours_1_d076bb311
CREATE TABLE __splink__filtered_neighbours_1_d076bb311 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__df_neighbours_c2e3b8aef as n
left join __splink__representatives_1_8c25a83e7 as l
on l.node_id = n.node_id
left join __splink__representatives_1_8c25a83e7 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
