"""empty message

Revision ID: 70f9339d5bd7
Revises: 595168173847
Create Date: 2020-02-08 16:23:52.223304

"""
from alembic import op
import sqlalchemy as sa
import sqlalchemy_utils


# revision identifiers, used by Alembic.
revision = '70f9339d5bd7'
down_revision = '595168173847'
branch_labels = None
depends_on = None


def upgrade():
    op.execute(sa.text("""
CREATE TYPE command_status AS ENUM ('sending', 'pending', 'failed_to_send', 'processing_error', 'done');
"""))
    # ### commands auto generated by Alembic - please adjust! ###
    op.add_column('command', sa.Column('status', sa.Enum('sending', 'pending', 'failed_to_send', 'processing_error', 'done', name='command_status'), nullable=False))
    # ### end Alembic commands ###


def downgrade():
    # ### commands auto generated by Alembic - please adjust! ###
    op.drop_column('command', 'status')
    # ### end Alembic commands ###
    op.execute(sa.text("""
DROP TYPE command_status;
"""))