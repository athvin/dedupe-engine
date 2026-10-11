-- templated: __splink__root_rows
-- physical: __splink__root_rows_a4a144ea6
CREATE TABLE __splink__root_rows_a4a144ea6 AS
select count(*) as count_of_edges_needing_processing
from __splink__filtered_neighbours_3_c78c4d25b
