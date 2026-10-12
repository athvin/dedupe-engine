-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_93b46f88e
CREATE TABLE __splink__agreement_pattern_counts_93b46f88e AS
select
gamma_dob,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_afa73af3d
group by gamma_dob, gamma_city, gamma_email
