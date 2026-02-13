import React from 'react';
import { Layout } from 'react-admin';
import { AppMenu } from './AppMenu';
import { AppAppBar } from './AppAppBar';
import { CreateElectionEventProvider, useCreateElectionEventStore } from '@/providers/CreateElectionEventContextProvider';
import { CreateDataDrawer } from '@/components/election-event/create/CreateElectionEventDrawer';
import { ImportDataDrawer } from '@/components/election-event/import-data/ImportDataDrawer';
import { CustomCssReader } from '@/components/CustomLayout'; // Keep this for CSS injection if needed

const LayoutWrapper = (props: any) => {
    const { createDrawer, closeCreateDrawer, openImportDrawer } = useCreateElectionEventStore();

    return (
        <>
            <CustomCssReader />
            <Layout
                {...props}
                menu={AppMenu}
                appBar={AppAppBar}
            />
            <CreateDataDrawer open={createDrawer} closeDrawer={() => closeCreateDrawer?.()} />
            <ImportDataDrawer
                title="electionEventScreen.import.eetitle"
                subtitle="electionEventScreen.import.eesubtitle"
                paragraph={"electionEventScreen.import.electionEventParagraph"}
            />
        </>
    );
};

export const AppLayout = (props: any) => {
    return (
        <CreateElectionEventProvider>
            <LayoutWrapper {...props} />
        </CreateElectionEventProvider>
    );
};
