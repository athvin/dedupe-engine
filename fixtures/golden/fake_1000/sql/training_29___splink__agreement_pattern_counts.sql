-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_bc6a45143
CREATE TABLE __splink__agreement_pattern_counts_bc6a45143 AS
select
gamma_dob,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_734e8609b
group by gamma_dob, gamma_city, gamma_email
