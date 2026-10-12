-- templated: __splink__root_rows
-- physical: __splink__root_rows_acc3f3ce1
CREATE TABLE __splink__root_rows_acc3f3ce1 AS
select count(*) as count_of_edges_needing_processing
from __splink__filtered_neighbours_3_0bdc3c331
