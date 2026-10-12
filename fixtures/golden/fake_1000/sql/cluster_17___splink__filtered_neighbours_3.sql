-- templated: __splink__filtered_neighbours_3
-- physical: __splink__filtered_neighbours_3_15d1deee9
CREATE TABLE __splink__filtered_neighbours_3_15d1deee9 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_2_b5b0993db as n
left join __splink__representatives_3_2eecc9e48 as l
on l.node_id = n.node_id
left join __splink__representatives_3_2eecc9e48 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
