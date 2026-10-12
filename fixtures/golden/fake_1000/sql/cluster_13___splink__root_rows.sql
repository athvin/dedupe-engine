-- templated: __splink__root_rows
-- physical: __splink__root_rows_9b167cf4d
CREATE TABLE __splink__root_rows_9b167cf4d AS
select count(*) as count_of_edges_needing_processing
from __splink__filtered_neighbours_2_b5b0993db
