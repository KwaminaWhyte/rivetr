-- The template submission queue was per-instance: an approved submission only
-- ever reached the instance that reviewed it, so it was removed from the
-- product. Drop the table; the approved rows it copied into service_templates
-- stay where they are.
DROP TABLE IF EXISTS community_template_submissions;
