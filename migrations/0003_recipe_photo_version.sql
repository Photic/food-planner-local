-- A token that changes whenever the photo does, so the image URL can change
-- with it.
--
-- Without this the URL is keyed only by recipe id, which leaves a choice
-- between caching a photo long enough to be useful and showing a replacement
-- promptly. Putting the version in the query string means a replaced photo is a
-- different URL, so the old one can be cached indefinitely and the new one is
-- never served from a stale entry.
--
-- Holds the upload time in milliseconds rather than a digest of the bytes: only
-- the change matters, and this avoids hashing megabytes on every upload. NULL
-- exactly when there is no photo, which is also how the listing reports one.
ALTER TABLE recipes ADD COLUMN photo_version INTEGER;

-- Any photo stored before this migration needs a version too, or it would be
-- indistinguishable from a recipe that has none.
UPDATE recipes
SET photo_version = CAST(strftime('%s', 'now') AS INTEGER) * 1000
WHERE photo IS NOT NULL;
