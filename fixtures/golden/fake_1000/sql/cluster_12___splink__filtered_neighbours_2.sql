-- templated: __splink__filtered_neighbours_2
-- physical: __splink__filtered_neighbours_2_b5b0993db
CREATE TABLE __splink__filtered_neighbours_2_b5b0993db AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_1_addbeeeaa as n
left join __splink__representatives_2_c6bd75d76 as l
on l.node_id = n.node_id
left join __splink__representatives_2_c6bd75d76 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
