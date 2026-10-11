-- templated: __splink__root_rows
-- physical: __splink__root_rows_1662d2d4c
CREATE TABLE __splink__root_rows_1662d2d4c AS
select count(*) as count_of_edges_needing_processing
from __splink__filtered_neighbours_2_59d5641c6
