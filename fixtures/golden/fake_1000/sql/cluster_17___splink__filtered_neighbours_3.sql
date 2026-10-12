-- templated: __splink__filtered_neighbours_3
-- physical: __splink__filtered_neighbours_3_0a9cd1b27
CREATE TABLE __splink__filtered_neighbours_3_0a9cd1b27 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_2_2669f7a2b as n
left join __splink__representatives_3_58f2cb423 as l
on l.node_id = n.node_id
left join __splink__representatives_3_58f2cb423 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
