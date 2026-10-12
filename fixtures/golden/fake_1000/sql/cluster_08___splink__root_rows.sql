-- templated: __splink__root_rows
-- physical: __splink__root_rows_b9ceed1cb
CREATE TABLE __splink__root_rows_b9ceed1cb AS
select count(*) as count_of_edges_needing_processing
from __splink__filtered_neighbours_1_d076bb311
