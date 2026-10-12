-- templated: __splink__filtered_neighbours_3
-- physical: __splink__filtered_neighbours_3_9942ce2cb
CREATE TABLE __splink__filtered_neighbours_3_9942ce2cb AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_2_3ad72a6ed as n
left join __splink__representatives_3_3343602e0 as l
on l.node_id = n.node_id
left join __splink__representatives_3_3343602e0 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
